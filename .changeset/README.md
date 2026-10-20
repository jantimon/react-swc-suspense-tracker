# Changesets

This folder holds [changesets](https://github.com/changesets/changesets): one markdown file
per pending change, describing the bump (patch/minor/major) and a changelog line.

Add one with `pnpm changeset`. On merge to `main`, the Release workflow opens a
"Version Packages" PR; merging that PR publishes to npm.
