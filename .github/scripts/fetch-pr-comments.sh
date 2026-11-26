#!/usr/bin/env bash
set -euo pipefail

# Fetches all pages of PR discussions via GraphQL for Claude review.
# Required env vars: GH_TOKEN, PR_NUMBER, REPO_OWNER, REPO_NAME
# Outputs: /tmp/pr_comments_context.txt

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

fetch_failed() {
  echo "Warning: Failed to fetch PR comments. Proceeding without comment context." >&2
  echo "⚠️ Unable to fetch existing comments due to API or data error." > /tmp/pr_comments_context.txt
  exit 0
}

# Each query has exactly one paginated connection, so gh can advance its cursor.
fetch_connection() {
  local query=$1 selector=$2 output=$3
  shift 3

  if ! gh api graphql --paginate --slurp -f query="$query" "$@" > "$WORK_DIR/pages.json"; then
    return 1
  fi

  # Reject missing connections and GraphQL errors instead of treating them as empty.
  jq -e --arg selector "$selector" '
    if length == 0 or any(.[]; (.errors // [] | length) > 0) then
      error("Missing pages or GraphQL errors")
    else
      map(getpath($selector | split(".")))
      | if any(.[]; .totalCount == null or (.nodes | type) != "array") then
          error("Missing connection data")
        else {totalCount: .[-1].totalCount, nodes: [.[].nodes[]]}
        end
    end' "$WORK_DIR/pages.json" > "$output"
}

for connection in comments reviewThreads reviews; do
  case "$connection" in
    comments) fields='author { login } body createdAt' ;;
    reviewThreads) fields='id isResolved isOutdated path line' ;;
    reviews) fields='author { login } body state createdAt' ;;
  esac
  QUERY='query($owner: String!, $repo: String!, $prNumber: Int!, $endCursor: String) {
    repository(owner: $owner, name: $repo) {
      pullRequest(number: $prNumber) {
        '"$connection"'(first: 100, after: $endCursor) {
          totalCount
          pageInfo { hasNextPage endCursor }
          nodes { '"$fields"' }
        }
      }
    }
  }'
  fetch_connection "$QUERY" "data.repository.pullRequest.$connection" "$WORK_DIR/$connection.json" \
    -f owner="$REPO_OWNER" -f repo="$REPO_NAME" -F prNumber="$PR_NUMBER" || fetch_failed
done

THREAD_QUERY='query($id: ID!, $endCursor: String) {
  node(id: $id) {
    ... on PullRequestReviewThread {
      comments(first: 100, after: $endCursor) {
        totalCount
        pageInfo { hasNextPage endCursor }
        nodes { author { login } body createdAt diffHunk }
      }
    }
  }
}'
jq -r '.nodes[].id' "$WORK_DIR/reviewThreads.json" > "$WORK_DIR/thread_ids"
: > "$WORK_DIR/thread_comments.jsonl"

while IFS= read -r thread_id; do
  fetch_connection "$THREAD_QUERY" data.node.comments "$WORK_DIR/comments_page.json" \
    -f id="$thread_id" || fetch_failed
  jq -c --arg id "$thread_id" '{key: $id, value: .}' "$WORK_DIR/comments_page.json" \
    >> "$WORK_DIR/thread_comments.jsonl" || fetch_failed
done < "$WORK_DIR/thread_ids"

# Fetch every page before selection. Bound compact JSON to 64 KiB, including
# metadata and its final newline. Prefer unresolved threads, then recent items.
if ! jq -cn \
    --slurpfile comments "$WORK_DIR/comments.json" \
    --slurpfile reviews "$WORK_DIR/reviews.json" \
    --slurpfile threads "$WORK_DIR/reviewThreads.json" \
    --slurpfile thread_comments "$WORK_DIR/thread_comments.jsonl" '
      def byte_size: tojson | utf8bytelength;
      def connection:
        {totalCount, retrievedCount: (.nodes | length), omittedCount: (.nodes | length), nodes: []};
      def omissions:
        .omittedCount = (.retrievedCount - (.nodes | length));
      def comment:
        {author: .author.login, body: ((.body // "")[:2000]), createdAt,
         bodyTruncated: ((.body // "" | length) > 2000)};
      def recent: sort_by(.createdAt) | reverse;
      def count_warning($label):
        select((.nodes | length) != .totalCount)
        | "Warning: \($label): retrieved \(.nodes | length) of \(.totalCount) items.";
      # Limit each thread so one long discussion cannot consume the whole file.
      def thread($comments):
        {path, line, isResolved, isOutdated, comments: ($comments | connection)}
        | reduce ($comments.nodes | recent | .[]) as $comment (.;
            (.comments.nodes += [($comment | comment + {
              diffHunk: ((.diffHunk // "")[:500]),
              diffHunkTruncated: ((.diffHunk // "" | length) > 500)
            })]) as $candidate
            | if ($candidate | byte_size) <= 12000 then $candidate else . end)
        | .comments |= omissions;
      def summarize:
        .comments |= omissions
        | .reviews |= omissions
        | .threads |= omissions
        | .limits.omittedThreadComments = (.limits.retrievedThreadComments
            - ([.threads.nodes[].comments.nodes | length] | add // 0));
      ($thread_comments | from_entries) as $by_thread
      | [$thread_comments[] | select((.value.nodes | length) != .value.totalCount)]
        as $incomplete_threads
      | [
          ($threads[0].nodes[] | . as $thread | $by_thread[.id] as $comments
            | {kind: "threads", priority: (if .isResolved then 2 else 0 end),
               createdAt: ([$comments.nodes[].createdAt] | max // ""),
               value: ($thread | thread($comments))}),
          ($comments[0].nodes[] | {kind: "comments", priority: 1, createdAt, value: comment}),
          ($reviews[0].nodes[] | {kind: "reviews", priority: 1, createdAt,
            value: (comment + {state})})
        ]
        # Sort descending by date within each priority group.
        | sort_by(.createdAt) | reverse | sort_by(.priority) as $candidates
      | {
          comments: ($comments[0] | connection),
          reviews: ($reviews[0] | connection),
          threads: ($threads[0] | connection),
          limits: {
            maxBytes: 65536, maxBodyCharacters: 2000, maxThreadBytes: 12000,
            retrievedThreadComments: ([$thread_comments[].value.nodes | length] | add // 0),
            omittedThreadComments: 0,
            notice: "Context is size-limited. Check omittedCount, omittedThreadComments, bodyTruncated and diffHunkTruncated. Unresolved threads and recent comments have priority."
          },
          warnings: [
            ($comments[0] | count_warning("comments")),
            ($reviews[0] | count_warning("reviews")),
            ($threads[0] | count_warning("threads")),
            ($incomplete_threads | select(length > 0)
              | "Warning: \(length) thread comment connections have retrieval count mismatches.")
          ]
        }
      | summarize
      | reduce $candidates[] as $item (.;
          (.[$item.kind].nodes += [$item.value] | summarize) as $candidate
          | if ($candidate | byte_size) < .limits.maxBytes then $candidate else . end)
      | if byte_size < .limits.maxBytes then . else error("Context exceeds size limit") end
      ' > /tmp/pr_comments_context.txt; then
  echo "⚠️ Unable to parse comment data." > /tmp/pr_comments_context.txt
else
  jq -r '.warnings[],
    (select(.comments.omittedCount + .reviews.omittedCount + .threads.omittedCount
      + .limits.omittedThreadComments > 0)
     | "Warning: Review context size limit omitted items; see counts in context.")' \
    /tmp/pr_comments_context.txt >&2
fi
