The purpose of this is to pin the release-please version
used by Nix when creating the release artifacts. We can't
just install the version within flake.nix as transitive
dependecies won't be pinned in that case.  