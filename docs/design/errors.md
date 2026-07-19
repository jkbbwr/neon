# Errors

`throw` / `try` / `catch`, the `Error` protocol, and what a failure carries. The rationale
for each choice is in `docs/decisions.md`; this is the shape.

## `Error`

    protocol Error for T {
        fn message(v: T) -> str
    }

`message` is the one thing an error knows about itself. It is deliberately *not* `Display`:
`to_string` answers "how does this value render" (`"Alice"`), `message` answers "what went
wrong" (`"failed to load user Alice"`). A type can be both a value and an error without the
two answers fighting over one method.

    record IoError { kind: :bad_fd | :eof }

    impl Error for IoError {
        fn message(e: IoError) -> str { "had an io error" }
    }

## `throws`

A `throws` clause takes **any type** — it is a claim about what a function can fail with,
not a claim that the failure is presentable:

    fn a() throws any
    fn b() throws :ok
    fn c() throws Person
    fn d() throws IoError | ParseError

`throws E ... where E: Error` covers generic propagation, because the clause resolves in the
function's rigid scope:

    fn retry[E](f: () -> i64 throws E) throws E where E: Error -> i64

## The one place `Error` is required

An error escaping `main`. That is the only point where the language must render something it
did not author, so it is the only point that demands the interface. Anything else is a
compile error:

    `i64` escapes `main`, which must report it, but it does not implement `Error`
    -- catch it, or give it an `impl Error` with a `message`

`main`'s channel is therefore **not a type** — it is a rule, checked per throw site
(`Throws::ImplicitError` in the checker). No existential is needed to enforce it: at every
escape point the concrete type is statically known, or is a concrete union, so `message` is
a direct call and a union is a tag switch. There is no vtable, no box, and nothing named
`Report`.

This holds precisely as long as **protocols stay bounds and never become types**. The day
`List[Error]` or `fn log(e: Error)` is expressible, a value of unknown type must be carried
at run time and a real protocol object is required.

## What a throw carries

The error slot inside the tagged result is compiler-owned and has no name in the language:

    { tag, union { ok: T, err: { error: E, ... } } }

Anything beyond the error value itself — a stacktrace, and later context — lives here rather
than on the user's type, because the error value cannot know it. A protocol default method
could not supply these either: a default can compute from fields the type already has, but
it cannot create storage.

## Not yet

- **Stacktrace.** *Important, and the largest missing piece.* The design reserves a slot for
  it in the error struct above, reached by a compiler-provided builtin rather than a
  protocol method — the value does not know where it was thrown. Nothing is implemented.

  What it needs: **frame capture at the point of `throw`**, which the runtime cannot do
  today. Traps `_exit` with no unwinding by design (`docs/decisions.md`), so there are no
  frames to walk. Two routes, neither cheap:
  - DWARF unwinding (`backtrace()` / libunwind). Interacts with the `_exit`-on-trap and
    allocator decisions, and needs the C emitted with frame pointers.
  - A shadow stack the compiler maintains, paid for on every call. Predictable and
    portable; costs the hot path. Note this one does not care about frame pointers at all.

  *The frame-pointer conflict is settled (2026-07-19) and is no longer a blocker.*
  `opt-release` trims the frame pointer; a stacktrace needs it. They are mutually exclusive
  and the trace wins: `--stacktrace`, or `stacktrace = true` under `[build]` in
  `neon.toml`, suppresses `-fomit-frame-pointer` and passes `-fno-omit-frame-pointer`
  instead. Explicitly, because `-O3` omits frame pointers on most targets on its own, so
  merely dropping the flag would not have been enough --
  `stacktrace_and_frame_pointer_omission_are_exclusive` in `cli/src/buildcfg.rs` pins both
  halves. The switch exists and does nothing else yet; what is unbuilt is the capture and
  the slot.

  Opt-in beyond that is a `mode` concern: traces on in `debug`, off in `release`. With
  traces off the slot is not in the layout, so it costs nothing.

  Open: whether frames record argument values (much more useful, much more capture cost),
  and whether a rethrow extends the original trace or starts a new one.

- **Context.** Deliberately unresolved. Every syntax considered (postfix clause, frame-level
  statement, `@context` annotation, scoped block) was rejected on feel, and it is worth
  noting the open question underneath: *a good stacktrace may make hand-written context
  largely redundant* — `load(path="/etc/cfg") → open → permission denied` says what
  `"while loading config"` would have. Settle stacktrace first; context may shrink to
  the exceptional case or disappear.

- **`throws 1` / `throws "test" | false`.** Singleton types do not exist anywhere in the
  language — the kinds are base bits, atoms, records, tuples, arrows. Pinned deliberately;
  it is a type-system feature, not an error-system one.
