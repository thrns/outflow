# Clean and Verbatim Mode Policy

**Status: policy decision required.** This document characterizes the current implementation and presents a proposed product policy. **PROPOSED — NOT APPROVED.** It does not change or supersede the authoritative requirements in [PRD §4, FR-07.1–FR-07.4](PRD.md#4-functional-requirements).

## Confirmed requirements

- Clean is the default cleanup mode ([FR-07.4](PRD.md#4-functional-requirements); the schema-v1 `CleanupSettings` default is `Clean`).
- Supported isolated filler deletion requires Clean mode and `remove_fillers: true` ([FR-07.2](PRD.md#4-functional-requirements), [T-036 adapter](../crates/settings/src/cleanup_policy.rs)).
- Verbatim disables filler deletion ([FR-07.2](PRD.md#4-functional-requirements)).
- Dictation commands are independently opt-in; only the supported “new line” and “new paragraph” phrases are recognized when enabled ([FR-07.3](PRD.md#4-functional-requirements), [UF-05](USER_FLOWS.md#uf-05--recognition-and-cleanup-f-06f-07)).
- Text processing is local and deterministic, with no inference LLM or network call in cleanup ([FR-07.1–FR-07.4](PRD.md#4-functional-requirements)).
- Vocabulary written replacements must remain protected from later punctuation formatting and filler deletion; the current span-aware pipeline preserves replacement bytes ([C-06](ARCHITECTURE.md#3-component-responsibilities), [core cleanup implementation](../crates/core/src/text_cleanup.rs)).

## Current implementation behavior

The schema-v1 settings adapter maps preferences to the existing core pipeline's filler flag using `mode == Clean && remove_fillers`. It does not select or skip the other stages.

| Stage | Clean | Verbatim |
|---|---|---|
| Whitespace normalization | Runs | Runs |
| Dictation commands | When enabled | When enabled |
| Vocabulary correction | Runs | Runs |
| Protected punctuation spacing | Runs | Runs |
| Filler removal | When `remove_fillers: true` | Disabled |

The current ordered stages are whitespace normalization → optional dictation commands → vocabulary correction → protected punctuation spacing → optional supported filler deletion. The adapter itself is pure and does not alter settings or persist transcripts. The React selector is a development preview and describes Verbatim as preserving filler words; it is not wired to this processing API ([selector](../src/app/TextCleanupModeSelector.tsx)).

## Unresolved product decisions

FR-07.4 describes Verbatim as preserving recognized wording “without cleanup transformations,” while FR-07.3 defines an ordered pipeline containing formatting, opt-in commands, and vocabulary correction. FR-07.2 only explicitly requires disabling filler deletion. Those requirements do not fully define which stages Verbatim skips. In particular, decide:

1. Does Verbatim skip additional formatting beyond the whitespace canonicalization required by the current pipeline?
2. Does a user-configured vocabulary correction still apply in Verbatim?
3. If dictation commands are explicitly enabled, do they still execute in Verbatim?

### Policy alternatives

| Alternative | Verbatim behavior | Consequences |
|---|---|---|
| **A. Literal transcript** | Normalize whitespace only; skip commands, vocabulary correction, punctuation spacing, and filler deletion. | Preserves recognized words most closely. An enabled command would not act, and explicit vocabulary choices would be ignored in this mode; explain that interaction in settings. |
| **B. Current pipeline** | Keep whitespace, explicitly enabled commands, vocabulary, and protected punctuation spacing; disable filler deletion only. | Matches the current adapter and keeps independent user choices effective. “Verbatim” would mean filler-preserving rather than transformation-free, which may conflict with FR-07.4's wording. |
| **C. User-directed exceptions (recommended)** | Normalize whitespace; run commands only when separately enabled; apply explicit vocabulary replacements; skip heuristic punctuation spacing and filler deletion. | Keeps explicit user-configured transformations available while making Verbatim skip heuristic formatting. It requires a precise definition of whether command substitution counts as a formatting transformation and revised acceptance coverage. |

### Recommendation — **PROPOSED — NOT APPROVED**

Adopt Alternative C for a future implementation decision: retain whitespace normalization for stable output, honor the separate opt-in for explicit dictation commands, preserve explicit vocabulary replacements, and skip heuristic punctuation spacing and filler deletion in Verbatim. This balances user-selected transformations with a narrower meaning of “verbatim,” but it is only a recommendation. No policy alternative is approved by this document, and the current implementation remains Alternative B until the maintainer resolves the decision and updates the authoritative requirements and tests together.

## Review required before implementation

The maintainer should approve one alternative or specify another policy, including the three questions above. Then reconcile PRD FR-07.1–FR-07.4, UF-05, C-06, the settings/cleanup contract, and T-036 acceptance tests in one coordinated change. Until that decision, this document is descriptive/proposed only; it authorizes no production behavior change.
