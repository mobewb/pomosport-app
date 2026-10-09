# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Global Rules

These rules apply to every task in this project unless explicitly overridden.
Bias: caution over speed on non-trivial work. Use judgment on trivial tasks.

## Rule 1 — Think Before Coding
State assumptions explicitly. If uncertain, ask rather than guess.
Present multiple interpretations when ambiguity exists.
Push back when a simpler approach exists.
Stop when confused. Name what's unclear.

## Rule 2 — Simplicity First
Minimum code that solves the problem. Nothing speculative.
No features beyond what was asked. No abstractions for single-use code.
Test: would a senior engineer say this is overcomplicated? If yes, simplify.

## Rule 3 — Surgical Changes
Touch only what you must. Clean up only your own mess.
Don't "improve" adjacent code, comments, or formatting.
Don't refactor what isn't broken. Match existing style.

## Rule 4 — Goal-Driven Execution
Define success criteria. Loop until verified.
Don't follow steps. Define success and iterate.
Strong success criteria let you loop independently.

## Rule 5 — Surface conflicts, don't average them
If two patterns contradict, pick one (more recent / more tested).
Explain why. Flag the other for cleanup.
Don't blend conflicting patterns.

## Rule 6 — Read before you write
Before adding code, read exports, immediate callers, shared utilities.
"Looks orthogonal" is dangerous. If unsure why code is structured a way, ask.

## Rule 7 — Checkpoint after every significant step
Summarize what was done, what's verified, what's left.
Don't continue from a state you can't describe back.
If you lose track, stop and restate.

## Rule 8 — Match the codebase's conventions, even if you disagree
Conformance > taste inside the codebase.
If you genuinely think a convention is harmful, surface it. Don't fork silently.

## Sub-Agent Strategy (Context Firewall)

The main agent **must not** perform direct codebase exploration, file searches, test runs, web lookups or code editing. All heavy-context work is delegated to sub-agents to keep the main context clean and focused on decision-making and user interaction.

### What to delegate

| Need                                                      | Delegate to                               | Output expected                                               |
| --------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------------- |
| Codebase exploration (layers, patterns, similar features) | **Explore agent**                         | Short summary of findings — max 20 lines                      |
| Detailed code structure (classes, interfaces, signatures) | **Explore agent**                         | Relevant files, interfaces, key signatures                    |
| Finding test patterns / existing test files               | **Explore agent**                         | Test file paths + brief description of patterns               |
| Library/API documentation lookup                          | **Explore agent** with web search         | Relevant API surface, constraints, integration notes          |
| Web research (best practices, comparisons)                | **General-purpose agent** with web search | Concise findings with source links                            |
| Running linter, typechecker, or tests                     | **General-purpose agent**                 | Pass/fail + list of errors to fix (no raw logs)               |
| Iterative fix-and-test loops                              | **General-purpose agent**                 | Final status: all passing, or remaining errors with file:line |

### Sub-agent protocol

1. **Strict scope:** A single, well-defined objective (e.g., "Identify all deprecated API calls in /src").
2. **Output format:** Request a specific structure (e.g., "Return a bullet list of findings, max 15 lines").
3. **No file modification:** Instruct sub-agents NOT to modify files unless explicitly requested.
4. **Context firewall:** Do not import raw logs, intermediate thought process, or verbose tool outputs. Only integrate the **final validated result**.
5. **Summarize:** Once the sub-agent completes, summarize the outcome in one sentence and resume the primary goal.
6. **Parallelism:** Launch independent sub-agents in parallel when possible (e.g., codebase exploration + web research).

## Memory

Use memory only to store current user preference. Else find another way to memorise need that will be shared by all teamate of thaat repository.

## Project Overview

## Common Commands

## Architecture

### Mobile App Structure

The app is layered, one-way only: `core → domain → infrastructure → app`.
`npm run arch` enforces it and CI blocks the merge on a violation.
**Read `mobile-app/docs/architecture.md` before moving code between these
directories** — it holds the permission matrix and the rules of the road.

```
mobile-app/
├── app/              # Expo Router routes ONLY (a route re-exports a screen)
│   └── (tabs)/       # Tab navigator routes
├── screens/          # Page components
├── components/       # UI components organized by feature
├── design-system/    # Tokens, atoms, molecules, theme
├── composition/      # Composition root (createContainer, providers)
├── infrastructure/   # Port implementations (ble/, storage/)
├── domain/           # Business logic, ports/ and modules/ — zero framework import
└── core/             # Pure primitives (observable, core/react/)
```

### Shared Libraries

## Testing


## CI/CD


## Module-Specific Notes