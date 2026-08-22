# Rationale

AI became a tool powerful enough to be useful in programming.
This has lead to a ton of advantages for exploratory work, research, prototyping or debugging for exemple.

It can even be used to write some production-ready code directly, provided the user actually verifies it.

And here is the whole point of this document: **humans are lazy by nature**.
Code that seemingly works tends to be accepted by people often because they do not feel like doing the extra mile and think about edge cases.

"AI will write tests" or "AI can be used in a feedback loop to improve the code" are among excuses we can often hear for this behaviour, as if tautological testing and/or willingly shipping buggy code into production on the premise that it will get fixed quickly is acceptable (it is not).

**AI is a productivity tool, not a human language compiler.**

# Rules

The following rules apply to all contributions made to Maestro.

- You MUST **fully** understand the code and documentation you author.
- Commit messages match actual changes.
- Contributions are tested and working as described.
- Documentation SHOULD NOT be AI-written (or at least, it should be modified to be human-friendly). AI documentation tends to be cumbersome. Our documentation is meant to be read by humans.
- Commit messages, issue messages and pull request messages MUST be hand written. AI-written issues and pull requests will be closed.
- Reviewing a PR with AI is forbidden. People can ask AI for a review themselves. [Don't be a meat proxy](https://gruhn.me/blog/2026-08-03/)
- Copyright infringement is forbidden.

Repeated failure to comply with those rules can result in being banned from further contributing to the project.

Regardless, you are **encouraged** to use AI for debugging or reviewing your own contributions before submitting.
