# strictsubst

A stricter `envsubst`: fails loud, listing every offending variable,
when a template references an environment variable that isn't set —
instead of `envsubst`'s default of silently substituting an empty
string. That silent-empty-string behavior is a real, recurring source
of subtle bugs in Docker entrypoint scripts and CI config templating
(a typo'd variable name doesn't error, it just quietly produces a
broken config). `envsubst` (from GNU gettext) also isn't installed by
default on minimal images like `alpine` without pulling in the whole
`gettext` package for one binary.

## Usage

```bash
strictsubst template.conf                       # -> stdout
strictsubst template.conf -o output.conf         # -> file
strictsubst -e vars.env template.conf            # layer extra vars from a dotenv file
cat template.conf | strictsubst                  # reads stdin if no file given
```

Template syntax:

```
listen ${PORT:-8080};                         # default if unset/empty
server_name ${SERVER_NAME};                   # required — fails if unset/empty
server_name ${SERVER_NAME:?must be set};      # required, with a custom error message
proxy_pass http://$UPSTREAM_HOST;             # bare $VAR form also works, always required
set $$literal_dollar_sign;                    # $$ -> literal $
```

```
$ strictsubst nginx.conf.template
strictsubst: 2 variable(s) failed:
  SERVER_NAME: SERVER_NAME must be set
  UPSTREAM_HOST: required but not set
```

Every missing variable is reported in one run, not just the first one
hit — no fix-one-rerun-find-the-next loop. Exit code `1` if anything
was missing, `0` on success.

## Status: built and verified against a realistic config template, both the failing and passing paths

- **18 unit tests**: template parsing (bare `$VAR` vs `${VAR}`,
  `${VAR:-default}`, `${VAR:?message}`, `$$` escaping to a literal
  `$`, a `$` not starting a valid reference left alone rather than
  erroring, an unterminated `${` kept literal instead of panicking, a
  default value that itself contains `:-` text taken literally after
  the first split), rendering (a set variable substitutes correctly,
  an unset required variable fails with a specific message, **all**
  missing variables reported together not just the first, a variable
  that's set-but-empty correctly treated the same as unset for
  `:-`/`:?` purposes, matching real shell parameter-expansion
  semantics), and the minimal dotenv loader.
- **Live-verified against a realistic nginx-config-style template**
  mixing every form at once (`${PORT:-8080}`, a required
  `${SERVER_NAME:?...}` with a custom message, a required bare
  `$UPSTREAM_HOST`-via-braces, and an escaped `$$price`): with no
  variables set, it correctly failed listing *both* real problems
  together (the custom SERVER_NAME message and the generic
  UPSTREAM_HOST message) while correctly *not* complaining about the
  two variables that have defaults. With variables supplied — first
  via real process environment variables, then again via `-e` loading
  them from a `.env` file instead — both produced byte-identical
  correct output, including `$$price` correctly rendering as the
  literal `$price`, not a variable reference. `-o` writing to a file
  and stdin input were separately verified working.
- Piping output into `head -2` exits cleanly (`0`) rather than
  panicking on a broken pipe — built in from the start this time,
  having already hit and fixed this exact bug live in two other tools
  in this collection (`netaudit`, `mailtrace`).

**Deliberate scope limits, stated plainly**: no nested/recursive
expansion (a `${VAR:-default}`'s default text is inserted verbatim,
even if it itself looks like `$OTHER_VAR` — real shell doesn't
recursively expand defaults either, matching the same behavior);
`:-`/`:?` are the only two operators supported — no `:=` (assign
default), `:+` (use alternate if set), or unquoted-vs-quoted
distinctions.
