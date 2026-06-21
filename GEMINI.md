@../GEMINI.md

# Engineering Standards for ruxel

## Rust Development

- **Use make:** Although rust usually uses cargo, we wrap it in a Makefile.
  Always confirm changes using `make test_release` which runs multiple cargo
  checks.
- **Warnings:** Do NOT ignore warnings from cargo checkers in the make output.
