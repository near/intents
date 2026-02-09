# OneshotCondVar

A oneshot condition variable smart contract for NEAR blockchain.

## Concept

This contract combines two Rust synchronization primitives:

1. **Oneshot Channel** - Can only be used once; after signaling, the contract self-destructs
2. **Condition Variable** - One party waits (`cv_wait`) while another notifies (`cv_notify_one`)

### Why "OneshotCondVar"?

- **Oneshot**: The contract can only complete one notification cycle, then cleans itself up
- **CondVar**: Exposes `cv_wait()` and `cv_notify_one()` semantics similar to `std::sync::Condvar`

## State Machine

```mermaid
stateDiagram-v2
    [*] --> Idle: new()

    state "Idle<br/>───<br/>No pending yield" as Idle
    state "WaitingForNotification<br/>───<br/>⏳ Yield pending" as WaitingForNotification
    state "Notified<br/>───<br/>No pending yield" as Notified
    state "Done<br/>───<br/>No pending yield" as Done

    Idle --> WaitingForNotification: cv_wait()<br/>(Creates yield promise)
    Idle --> Notified: cv_notify_one()<br/>(No yield to resume)

    WaitingForNotification --> Notified: cv_notify_one()<br/>(Resumes yield)
    WaitingForNotification --> Idle: cv_wait_resume() timeout<br/>(Emits Timeout, can retry)

    Notified --> Done: cv_wait()<br/>(Immediate success)
    Notified --> Done: cv_wait_resume()<br/>(Yield resumed)

    Done --> [*]: cleanup
```

## States

| State | Description |
|-------|-------------|
| `Idle` | Initial state. Ready for `cv_wait()` or `cv_notify_one()` |
| `WaitingForNotification` | `cv_wait()` called, yield promise active waiting for notification |
| `Notified` | `cv_notify_one()` called, notification received |
| `Done` | Terminal state, triggers contract cleanup |

## State Transitions

| From State | Method | To State | Notes |
|------------|--------|----------|-------|
| `Idle` | `cv_wait()` | `WaitingForNotification` | Creates yield promise |
| `Idle` | `cv_notify_one()` | `Notified` | No yield to resume |
| `WaitingForNotification` | `cv_notify_one()` | `Notified` | Resumes yield (may fail if timed out) |
| `WaitingForNotification` | `cv_wait_resume()` timeout | `Idle` | Emits `Timeout` event, can retry |
| `Notified` | `cv_wait()` | `Done` | Immediate success, no yield |
| `Notified` | `cv_wait_resume()` | `Done` | Yield resumed or race condition |

## Action Matrix

What happens when each action is performed in each state:

| State | `cv_wait()` | `cv_notify_one()` | Promise resumed (timeout) | Promise resumed (notified) |
|-------|-------------|-------------------|---------------------------|----------------------------|
| **Idle** | → `WaitingForNotification`<br/>Creates yield promise | → `Notified`<br/>No yield to resume | ❌ Impossible<br/>No pending promise | ❌ Impossible<br/>No pending promise |
| **WaitingForNotification** | ❌ Error<br/>"already waiting" | → `Notified`<br/>Resumes yield | → `Idle`<br/>Emits Timeout, can retry | → `Done`<br/>Returns `true` |
| **Notified** | → `Done`<br/>Immediate success | ❌ Error<br/>"already notified" | → `Done`<br/>Race condition handled | → `Done`<br/>Returns `true` |
| **Done** | ❌ Error<br/>"already done" | ❌ Error<br/>"already notified" | ❌ Impossible<br/>Already terminal | ❌ Impossible<br/>Already terminal |

## Execution Paths

### Path 1: cv_wait() then cv_notify() (Happy path)

```
Idle ──cv_wait()──► WaitingForNotification ──cv_notify_one()──► Notified ──cv_wait_resume()──► Done
```

1. `cv_wait()`: Creates yield promise, state → `WaitingForNotification`
2. `cv_notify_one()`: Resumes yield, state → `Notified`
3. `cv_wait_resume()` callback: State → `Done`, returns `true`

### Path 2: cv_wait() Timeout

```
Idle ──cv_wait()──► WaitingForNotification ──timeout──► Idle (can retry)
```

1. `cv_wait()`: Creates yield promise, state → `WaitingForNotification`
2. Yield times out
3. `cv_wait_resume()` callback: State → `Idle`, emits `Timeout`, returns `false`

### Path 3: cv_notify() then cv_wait() (Notify first)

```
Idle ──cv_notify_one()──► Notified ──cv_wait()──► Done
```

1. `cv_notify_one()`: State → `Notified`
2. `cv_wait()`: Immediate success, state → `Done`, returns `true`

### Path 4: Race Condition (Timeout + Late Notification)

```
Idle ──cv_wait()──► WaitingForNotification ──(yield times out)──► cv_notify_one() ──► Notified ──cv_wait_resume()──► Done
```

1. `cv_wait()`: State → `WaitingForNotification`
2. Yield times out internally (callback not yet executed)
3. `cv_notify_one()` arrives: `yield.resume()` fails, but state → `Notified`
4. `cv_wait_resume()` callback: Sees `Notified` → `Done`, returns `true`

## Error Conditions

| Current State | Method | Error |
|---------------|--------|-------|
| `WaitingForNotification` | `cv_wait()` | "already waiting for notification" |
| `Done` | `cv_wait()` | "already done" |
| `Notified` | `cv_notify_one()` | "already notified" |
| `Done` | `cv_notify_one()` | "already notified" |

## API

### `cv_state() -> &Status`
Returns the current `Status` of the condition variable (`Idle`, `WaitingForNotification`, `Notified`, or `Done`).

### `cv_view() -> &State`
Returns the full `State` including configuration (`Config`) and current status.

### `cv_wait() -> PromiseOrValue<bool>`
Called by the `waiter` to wait for notification. Returns:
- `true` if notification received (state becomes `Done`)
- `false` if timeout occurred (state resets to `Idle`, can retry)

### `cv_notify_one()`
Called by the `notifier_id` to signal notification. Wakes up any waiting `cv_wait()`.

### `cv_is_notified() -> bool`
Returns `true` if state is `Notified` or `Done`.

## Usage Pattern

```
Party A (waiter)          Contract              Party B (notifier_id)
       |                         |                         |
       |------- cv_wait() ------>|                         |
       |                    [WaitingForNotification]       |
       |                         |<--- cv_notify_one() ----|
       |                    [Notified]                   |
       |<-- cv_wait_resume() ----|                         |
       |      returns true  [Done]                         |
       |                    [cleanup]                      |
```
