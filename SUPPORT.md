# Support

## Getting help

guroku is a pre-1.0 project. It is built and maintained by one person in
the open. There is no commercial support offering, no paid tier, and no
service-level agreement of any kind. Treat everything below as best
effort.

If you need guaranteed support windows, guroku is not currently the
right tool for you.

## Where to ask

Pick the channel that matches what you actually have. Picking the wrong
one makes things slower for everyone.

- **Questions** ("how do I do X", "is this expected", "what does this
  error mean") go to GitHub Discussions:
  https://github.com/nktkt/guroku/discussions. Search first; the answer
  is often already there.
- **Bug reports** go to GitHub Issues using the `bug_report` template.
  A bug is reproducible behavior that contradicts the documentation or
  is obviously wrong (panics, corrupted lockfiles, wrong resolution
  result, etc.).
- **Feature requests** go to GitHub Issues using the `feature_request`
  template. Before opening one, read ROADMAP.md. If the feature is
  already listed there, add a comment on the existing issue (or open a
  Discussion) instead of filing a duplicate.
- **Security vulnerabilities** must NOT be filed as public issues or
  posted in Discussions. File a private advisory at
  https://github.com/nktkt/guroku/security/advisories/new. The full
  policy and disclosure timeline live in SECURITY.md.

## Before opening an issue

A good bug report saves several round trips. Please include all of the
following:

1. The output of `guroku --version`.
2. A relevant slice of debug logs from your failing command. Re-run it
   with `GUROKU_LOG=debug guroku <your-command>` and paste the lines
   around the failure. Do not paste megabytes; the relevant slice is
   usually 20 to 100 lines.
3. Confirmation that you are on the latest released version. If you
   built from source, include the output of `git log -1 --oneline` so
   it is clear which commit you are on.
4. The exact command you ran, the operating system and architecture,
   and a minimal reproduction if you can produce one.

Issues missing this information will usually be closed with a request
to refile.

## What you should NOT use issues for

The issue tracker is a working surface, not a public forum. Please do
not use it for:

- Venting, frustration posts, or low-content "this is broken" reports
  with no reproduction.
- Generic "is this a good idea" or "should I use guroku" questions.
  Those belong in Discussions.
- npm, Node.js, or JavaScript usage questions that are not about
  guroku itself. Stack Overflow and the Node ecosystem's own forums
  are better for those.
- Requests for parity with specific npm CLI flags or behaviors purely
  on the grounds that npm has them. guroku is not trying to be a
  drop-in clone; intentional divergences are documented in
  ARCHITECTURE.md and ROADMAP.md.

Issues that fall into the categories above will be closed, usually
with a short pointer to this document.

## Response times

Response times are best effort. The maintainer is one person with a
day job. Realistic expectations:

- Bugs that block install correctness for a real project (wrong
  resolution, lockfile corruption, data loss, security issues) get
  the fastest attention.
- Other bugs are triaged when there is time.
- Feature requests may sit open for a long time. That is not a
  rejection; it is a queue.
- Discussions threads are read but not always answered the same day.

If an issue has gone quiet for more than a few weeks and you believe
it is important, a polite bump is fine. Repeated bumps are not.

## Commercial support

There is no commercial support offering today. There is no support
email and one is intentionally not provided; private one-to-one
support is not something the project can sustain at this stage. If
that changes, it will be announced in the README and in CHANGELOG.md.

For anything that genuinely must be private, use the security
advisory channel linked above. Do not use it for non-security
questions.
