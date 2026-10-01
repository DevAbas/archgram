# Releasing archgram

A release is a tag. Pushing `vX.Y.Z` runs the Release workflow
(`.github/workflows/release.yml`): archgram is built for every platform on a
runner of that platform, each binary draws every example and the drawings
must match byte for byte, the npm packages are assembled and tested, and
then staged on npm with provenance through trusted publishing. A staged
package goes public when you approve it on npm with two-factor
authentication.

## Every release

`main` takes changes only through a pull request whose CI passed, so the
version goes in by a pull request and the tag follows the merge.

1. On a branch `release/X.Y.Z`: set the version in `Cargo.toml`
   (`[workspace.package]`; the npm packages take theirs from it), and in
   `CHANGELOG.md` turn `Unreleased` into the version and today's date, and
   point its compare link at the new tag. In `README.md`, point the skill's
   install command at the new version. Commit (`chore: set the version to
   X.Y.Z`).
2. Push the branch, open a pull request titled as its commit, and merge it
   with **Squash and merge** once CI passes.
3. Tag the merged commit on `main` and push the tag:

   ```sh
   git switch main
   git pull --ff-only
   git tag -a vX.Y.Z -m "archgram X.Y.Z"
   git push origin vX.Y.Z
   ```

4. Watch the Release workflow. Its publish job waits for your approval (the
   `npm` environment); approve it once the build and package jobs pass. If
   a job fails, see [When a release fails](#when-a-release-fails).
5. On npmjs.com, in the **Staged Packages** tab, approve the seven staged
   packages: the six `@archgram/cli-*` first, `archgram` last, so the
   launcher never points at a version that is not public yet. Each approval
   asks for two-factor authentication.
6. The workflow drafts the GitHub release: an archive of the skill, its
   checksum, a signed record of the build that made it, and the version's
   section of the changelog as its notes. Read it, then publish it;
   published, an immutable release keeps its tag and files as they are.
7. Check that the release is whole: every package at the new version on
   npm, and the skill's archive verified.

   ```sh
   npm view archgram@X.Y.Z version
   npm view @archgram/cli-linux-x64@X.Y.Z version
   gh attestation verify archgram-skill-X.Y.Z.tar.gz -R byAbas/archgram
   ```

## When a release fails

What to do depends on whether anything has reached users.

- **Nothing is public yet** (a job failed, or packages are staged but not
  approved): reject any staged package on npm, delete the draft GitHub
  release and the tag, and fix the cause through a pull request. If no
  package was staged, tag the new `main` with the same version. If any
  was, release the next patch version: npm keeps a staged version even
  once it is rejected, and will not stage it again ("Cannot stage
  previously published version", 0.6.0).
- **Some packages are public:** an npm version can never be published
  again. If the public packages are sound, approve the rest; if not, fix
  through a pull request and release the next patch version.
- **A public version is broken:** deprecate it on npm with the reason
  (`npm deprecate archgram@X.Y.Z "<reason>"`), and release the next patch
  version with the fix. A published GitHub release stays as it is.

## The first release, once

npm lets a package take a trusted publisher only once the package exists,
so the first version of each package is published by hand, from the files
the workflow built. Nothing is built on your machine.

1. On npm: the `archgram` organization exists, and your account has
   two-factor authentication for authorization and publishing.
2. Push the tag as above. The workflow builds and packs, and skips
   publishing, since `NPM_TRUSTED_PUBLISHING` is not set yet.
3. From the workflow run's summary, download the `npm-packages` artifact
   and unzip it: seven `.tgz` files.
4. Publish them, the platform packages first:

   ```sh
   npm login
   for package in archgram-cli-*.tgz; do npm publish "$package" --access public; done
   npm publish archgram-0.3.0.tgz
   ```

5. On npmjs.com, for each of the seven packages, open **Settings**, then
   **Trusted Publisher**, choose **GitHub Actions** and enter: organization
   or user `byAbas`, repository `archgram`, workflow `release.yml`,
   environment `npm`. Leave **Allow npm publish** unticked: the publisher
   may only stage, as npm recommends (docs.npmjs.com, Trusted publishing).
6. On each package's **Settings**, under **Publishing access**, choose
   **Require two-factor authentication and disallow tokens**.
7. On GitHub, in the repository's **Settings**: under **Environments**,
   create `npm`, with you as its required reviewer and only `v*` tags
   allowed to deploy to it;
   under **Secrets and variables**, then **Actions**, then **Variables**,
   add `NPM_TRUSTED_PUBLISHING` with the value `true`.

From then on every tag stages its packages by itself, with no token stored
anywhere, and each goes public on your approval.
