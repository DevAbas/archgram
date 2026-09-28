# Releasing archgram

A release is a tag. Pushing `vX.Y.Z` runs the Release workflow
(`.github/workflows/release.yml`): archgram is built for every platform on a
runner of that platform, each binary draws every example and the drawings
must match byte for byte, the npm packages are assembled and tested, and
then published to npm with provenance through trusted publishing.

## Every release

1. Set the version in `Cargo.toml` (`[workspace.package]`); the npm
   packages take theirs from it.
2. In `CHANGELOG.md`, turn `Unreleased` into the version and today's date,
   and add its compare link at the bottom.
3. Commit (`chore: version X.Y.Z`), then tag and push:

   ```sh
   git tag -a vX.Y.Z -m "archgram X.Y.Z"
   git push origin main vX.Y.Z
   ```

4. Watch the Release workflow. If it fails, nothing is published; fix,
   delete the tag, and tag again.
5. Create the GitHub release from the tag, with the version's section of
   the changelog.

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
   or user `DevAbas`, repository `archgram`, workflow `release.yml`,
   environment `npm`.
6. On each package's **Settings**, under **Publishing access**, choose
   **Require two-factor authentication and disallow tokens**.
7. On GitHub, in the repository's **Settings**: under **Environments**,
   create `npm` (and, if you like, require your approval before it runs);
   under **Secrets and variables**, then **Actions**, then **Variables**,
   add `NPM_TRUSTED_PUBLISHING` with the value `true`.

From then on every tag publishes by itself, with no token stored anywhere.
