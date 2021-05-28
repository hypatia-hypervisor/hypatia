# How to Contribute

Thank you for your interest in contributing to the Hypatia
hypervisor project!

We'd love to accept your patches and contributions to this
project.  There are just a few small guidelines you need to
follow.

## DCO

Contributions to this project must be made in accordance with
the Developer Certificate of Origin (DCO): this is simply an
affirmation that you have the right to submit the change and
understand that it will be distributed by the project under
the project's license.

Note that the project as a whole is licensed under the MIT license
(see the LICENSE file for details).  Contributions to Hypatia must
be compatibly licensed, and in lieu of a specific compatible
license for any given change, that change must be licensed with the
MIT license.

You don't need to do anything formal to sign onto the DCO, but
you must sign your commits to acknowledge that you have read it
and are complying with its terms.

The text of the DCO is available in `DCO` and online at
https://developercertificate.org/.

## Code reviews

All submissions, including submissions by project members,
require review by both a contributor and an owner; these can
be the same person.

All software in the project has a set of owners and owners are
indicated by email addresses in files named 'OWNERS' scattered
throughout the directory tree.  All owners are contributors.

Owner relationships are hierarchical: the immediate owners of a
particular component are those contributers listed in the
nearest OWNERS file; OWNERS in higher files similarly own
software further down in the tree.

The owners mechanism allows contributors to specialize in
particular components of the overall system, lending their
expertise to the review of code going into those systems.  Thus,
in order to submit, one should have an owner's approval.

We use either GitHub pull requests or Gerrit for reviews.
Consult [GitHub Help](https://help.github.com/articles/about-pull-requests/)
for more information on using pull requests.

## What Constitutes a Good Contribution?

A "good" contribution will consist of a set of git commits that
"tell a story" in their progression.  We use a rebase-to-main
style of incorporating changes so your commits will be
incorporated into the repository in order.  Please strive to
make commits clean, discrete, and reasonably sized.

New code should be accompanied by tests, if possible.

## Submission Requirements

All code should be formatted using the `rustfmt` tool (installed
from cargo).  New source files should start with the intellectual
property boilerplate; see the BOILERPLATE.* files in the `lib/`
directory for templates.

All submissions must be signed-off by the author; do this by using
`git commit -s`.
