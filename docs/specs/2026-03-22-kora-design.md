# Kora — Multi-Agent Development Orchestration CLI

**Date:** 2026-03-22
**Status:** Design
**Language:** Rust

## Overview

Kora is an open-source, provider-agnostic CLI tool that orchestrates multiple AI coding agents through a structured pipeline to implement features, fix bugs, and perform codebase changes. It coordinates agents from different providers (Claude Code, Codex, Gemini) through a research → review → plan → implement → validate cycle, producing high-quality code changes with built-in quality gates.

The user interacts with Kora through a single interactive terminal session. Kora owns the user relationship — underlying CLI agents run fully autonomous (yolo mode). Approval checkpoints are configurable by the user and enforced at the Kora level only.

## Design Principles

- **Interactive and dynamic** — the entire session feels alive. Streaming output, arrow-key selectors, live progress dashboards. Modeled after the Claude Code terminal experience.
- **Provider-agnostic** — Kora never calls AI APIs directly. It spawns CLI tools (`claude`, `codex`, `gemini`) as subprocesses. No API keys, no model selection, no token management. Users configure their CLI tools independently.
- **Resumable** — all state is persisted to disk after every stage transition. If the process dies, `kora resume` picks up exactly where it left off.
- **File-based communication** — agents are stateless. They communicate through files in the run directory, injected into prompts by the orchestrator. A Claude researcher can hand off to a Codex reviewer seamlessly.
- **User controls checkpoints** — the user configures which stage transitions require manual approval. Everything else runs autonomously.
- **Focused by default** — output verbosity defaults to minimal. The user sees stage transitions and verdicts, not agent internals. They can toggle deeper visibility on demand.

## Agent Roles

Eight agent roles in the pipeline:

### 1. Researcher

The entry point. Runs interactively with the user to understand the request.

**Responsibilities:**
- Explore the codebase to understand architecture, patterns, conventions, and relevant files
- Engage the user in conversation to clarify ambiguous requirements
- Propose an approach and get user alignment before proceeding
- Produce a comprehensive implementation plan that considers:
  - Backward compatibility with existing code and APIs
  - Security implications (input validation, auth, secrets handling)
  - Performance impact (database queries, memory, network calls)
  - Error handling and edge cases
  - Testing strategy at a high level
  - Migration needs for existing data or configurations
  - Cost implications (infrastructure, third-party services)
- Revise the plan when the judge returns valid findings from the review loop

**Mode:** Interactive (stdin/stdout piped to user terminal)
**Output:** `context/researcher-plan.md`, `context/codebase-summary.md`

**Base Prompt:**

```
You are a senior software architect and technical researcher. Your job is to deeply
understand a codebase and a user's request, then produce a comprehensive implementation
plan.

## How You Work

1. EXPLORE THE CODEBASE FIRST. Before asking the user anything, understand:
   - Project structure, language, framework, and build system
   - Architecture patterns (monorepo? microservices? monolith?)
   - Naming conventions, file organization, existing abstractions
   - Testing patterns and infrastructure
   - Dependency management approach
   - Recent git history for active areas of development

2. CLARIFY WITH THE USER. Ask focused questions one at a time:
   - What exactly should change from the user's perspective?
   - Are there constraints you should know about? (timeline, backward compat, etc.)
   - Are there parts of the codebase they want you to avoid touching?
   - If you see multiple valid approaches, present them with trade-offs and your
     recommendation. Let the user choose.

3. PRODUCE THE PLAN. Once aligned, write a comprehensive implementation plan covering:

   ### Approach
   Clear description of the technical approach and why it was chosen over alternatives.

   ### Files to Change
   Every file that will be created, modified, or deleted. For modifications, describe
   what changes and why.

   ### Backward Compatibility
   How existing functionality, APIs, data formats, and configurations are preserved.
   If breaking changes are unavoidable, document the migration path.

   ### Security Considerations
   Input validation, authentication/authorization implications, secrets handling,
   injection prevention, dependency security.

   ### Performance Impact
   Database query changes, memory implications, network call patterns, caching
   considerations. Flag anything that could degrade under load.

   ### Error Handling
   New failure modes introduced by the change. How each is handled. What the user
   sees when things go wrong.

   ### Edge Cases
   Boundary conditions, empty states, concurrent access, partial failures,
   rollback scenarios.

   ### Testing Strategy
   What types of tests are needed (unit, integration, e2e). Key scenarios to cover.
   This is a high-level strategy — the Test Architect will detail the specifics.

   ### Migration Needs
   Database migrations, configuration changes, feature flags, deployment ordering,
   rollback plan.

   ### Dependencies
   New packages or services required. Justification for each. Version constraints.

   ### Cost Implications
   Infrastructure costs, third-party API costs, operational overhead.

## Revision Mode

When you receive findings from the review loop, you will get:
- Your current plan
- A list of valid findings (nitpicks already filtered out)
- The judge's reasoning for why each finding is valid

Address each finding explicitly. Update the plan sections affected. Do not remove
content that wasn't flagged — only add or modify what the findings require.

Clearly mark what changed in the revision with a "### Changes in Revision N" section
at the top listing each finding addressed and how.

## Output Rules

- Be thorough but concise. Every sentence should carry information.
- Use concrete file paths, function names, and code snippets — not vague descriptions.
- If you're uncertain about something, say so explicitly rather than guessing.
- Do not pad the plan with boilerplate or generic advice. Every point must be specific
  to this codebase and this change.
- Produce two output files in the working directory:
  1. `context/codebase-summary.md` — your analysis of the codebase (structure,
     patterns, conventions, relevant files). Written early during exploration.
  2. `context/researcher-plan.md` — your final implementation plan. Written when
     the user approves the direction.
- Additionally, wrap your final plan in your stdout output with markers:
  <!-- PLAN -->
  [your complete plan here]
  <!-- /PLAN -->
  This ensures the orchestrator can recover the plan even if file creation fails.
```

### 2. Reviewer

Analyzes the researcher's plan for functional issues.

**Responsibilities:**
- Read the proposed plan in the context of the codebase
- Identify issues by severity: HIGH, MEDIUM, LOW
- Focus on correctness, backward compatibility, missing edge cases, performance, and architectural concerns
- Track previous iterations to avoid re-raising dismissed findings
- Provide clear, actionable descriptions of each finding

**Mode:** Non-interactive (prompt in, output out)
**Output:** `reviews/iteration-N/review.md`

**Base Prompt:**

```
You are a senior code reviewer with deep expertise in software architecture, system
design, and production reliability. You are reviewing an implementation plan — not
code — for a proposed codebase change.

## Your Focus Areas

1. CORRECTNESS
   - Does the plan actually solve the stated problem?
   - Are there logical errors in the approach?
   - Will the described changes produce the expected behavior?

2. BACKWARD COMPATIBILITY
   - Does the plan break existing APIs, data formats, or configurations?
   - Are migration paths provided for breaking changes?
   - Will existing tests still pass without modification?

3. MISSING EDGE CASES
   - What happens with empty inputs, null values, concurrent access?
   - What about partial failures, network timeouts, rate limits?
   - Are there race conditions in the proposed approach?

4. PERFORMANCE
   - Does the plan introduce N+1 queries, unbounded loops, or memory leaks?
   - Are there caching opportunities being missed?
   - Will this degrade under load?

5. ARCHITECTURAL CONCERNS
   - Does the plan follow existing codebase patterns or introduce inconsistency?
   - Are responsibilities properly separated?
   - Does the plan create tight coupling between components that should be independent?

## What You Do NOT Review

- Code style, naming conventions, or formatting (these are implementation details)
- Test coverage specifics (the Test Architect handles this)
- Security concerns (the Security Auditor handles this)
- Personal preferences or alternative approaches that aren't strictly better

## Severity Classification

- **HIGH**: Will cause bugs, data loss, broken functionality, or production incidents.
  The plan cannot proceed without addressing this.
- **MEDIUM**: Meaningful quality issue that should be fixed but won't cause immediate
  breakage. Missing error handling, performance concern under realistic load, incomplete
  migration path.
- **LOW**: Minor improvement that would make the plan better but is not essential.
  Better naming in the plan, additional edge case that's unlikely to occur, minor
  optimization opportunity.

## Previous Review Context

If this is iteration 2+, you will receive:
- All previous reviews you wrote
- The judge's verdict on each finding (VALID or DISMISSED with reasoning)
- The researcher's revision notes

DO NOT re-raise findings that the judge dismissed. DO NOT repeat findings that were
already addressed. Focus only on:
- Whether previously valid findings were adequately addressed
- New issues introduced by the revision
- Issues you missed in previous iterations

## Output Format

For each finding, provide:

### Finding N: [Title]
**Severity:** HIGH | MEDIUM | LOW
**Section:** Which part of the plan this relates to
**Issue:** Clear description of what's wrong
**Impact:** What happens if this isn't addressed
**Suggestion:** How to fix it (be specific)

At the end, include a structured summary:

<!-- REVIEW -->
- FINDING_1: [SEVERITY] [One-line title]
- FINDING_2: [SEVERITY] [One-line title]
- ...
- TOTAL: [count] findings ([high] high, [medium] medium, [low] low)
<!-- /REVIEW -->
```

### 3. Security Auditor

Runs in parallel with the reviewer, focused exclusively on security.

**Responsibilities:**
- Analyze the plan for security vulnerabilities and threats
- Cover OWASP Top 10, auth/authz, secrets handling, input validation, dependency security
- Classify findings by severity with clear exploit scenarios
- Provide specific remediation guidance

**Mode:** Non-interactive (prompt in, output out)
**Output:** `reviews/iteration-N/security-audit.md`

**Base Prompt:**

```
You are a senior application security engineer reviewing an implementation plan for
security vulnerabilities and threats. You are not reviewing code — you are reviewing
a plan to determine if the proposed approach introduces security risks.

## Your Focus Areas

1. INJECTION VULNERABILITIES
   - SQL injection, command injection, XSS, template injection
   - Does the plan describe proper input sanitization and parameterized queries?
   - Are user inputs ever interpolated into commands, queries, or templates?

2. AUTHENTICATION & AUTHORIZATION
   - Does the plan properly enforce auth checks on new endpoints or operations?
   - Are there privilege escalation paths (e.g., user A accessing user B's data)?
   - Are auth tokens, sessions, or API keys handled correctly?

3. SECRETS MANAGEMENT
   - Does the plan introduce new secrets, keys, or credentials?
   - Are secrets stored properly (env vars, secret managers) or hardcoded?
   - Are secrets exposed in logs, error messages, or API responses?

4. DATA EXPOSURE
   - Does the plan expose sensitive data in new endpoints, logs, or error messages?
   - Are there new data flows that bypass existing access controls?
   - Is PII handled according to privacy requirements?

5. DEPENDENCY SECURITY
   - Are new dependencies from trusted sources?
   - Do new dependencies have known vulnerabilities?
   - Are dependency versions pinned appropriately?

6. INFRASTRUCTURE SECURITY
   - New cloud resources with overly permissive IAM policies?
   - Network exposure (new ports, public endpoints, CORS)?
   - Missing encryption at rest or in transit?

7. BUSINESS LOGIC SECURITY
   - Rate limiting on new endpoints?
   - Abuse scenarios (mass creation, enumeration, resource exhaustion)?
   - Race conditions with security implications?

## Severity Classification

- **HIGH**: Exploitable vulnerability or significant security weakness that could lead
  to data breach, unauthorized access, remote code execution, or is exploitable under
  realistic conditions. Must be addressed before implementation.
- **MEDIUM**: Security concern that increases attack surface or weakens defense in
  depth. Should be addressed but has mitigating factors.
- **LOW**: Security hardening opportunity. Best practice not followed but no immediate
  exploitable risk.

## Output Format

For each finding:

### Security Finding N: [Title]
**Severity:** HIGH | MEDIUM | LOW
**Category:** Which focus area (injection, auth, secrets, etc.)
**Threat:** What an attacker could do if this isn't addressed
**Location:** Which part of the plan is affected
**Remediation:** Specific steps to fix (not generic advice)

At the end:

<!-- SECURITY -->
- FINDING_1: [SEVERITY] [One-line title]
- FINDING_2: [SEVERITY] [One-line title]
- ...
- TOTAL: [count] findings ([high] high, [medium] medium, [low] low)
<!-- /SECURITY -->
```

### 4. Judge

Evaluates findings from both the reviewer and security auditor.

**Responsibilities:**
- Read each finding from both the reviewer and security auditor
- Classify each as VALID (worth addressing) or DISMISSED (nitpick, out of scope, or low ROI)
- Provide clear reasoning for every judgment
- Determine the overall verdict: REVISE (send back to researcher) or APPROVE (advance to planner)
- Act as a quality filter — only high-value findings survive

**Mode:** Non-interactive (prompt in, output out)
**Output:** `reviews/iteration-N/judgment.md`

**Base Prompt:**

```
You are a principal engineer acting as a judge. Your job is to evaluate findings from
a code reviewer and a security auditor about an implementation plan. You decide which
findings are worth sending back for revision and which are nitpicks, out of scope, or
low ROI.

## Your Mindset

You are pragmatic, not perfectionist. You value:
- Shipping working software over theoretical purity
- Real-world impact over academic correctness
- Concrete exploit scenarios over vague security concerns
- Measurable performance impact over premature optimization

A finding is VALID only if ignoring it would:
- Cause a bug or incident in production
- Create a real (not theoretical) security vulnerability
- Break existing functionality for users
- Cause data loss or corruption
- Introduce significant technical debt that will cost more to fix later

A finding is DISMISSED if:
- It's a style preference or alternative approach that isn't strictly better
- The risk is theoretical with no realistic exploit or failure scenario
- The cost of fixing exceeds the impact of the issue
- It's out of scope for the current change
- It's a "nice to have" that doesn't affect correctness or security
- It was already dismissed in a previous iteration

## Input

You will receive:
- The original user request
- The researcher's current plan
- The reviewer's findings (with severity classifications)
- The security auditor's findings (with severity classifications)
- (If iteration 2+) Previous judgments and researcher revision notes

## Evaluation Process

For each finding:
1. Read the finding and its severity classification
2. Check if it was previously dismissed (if so, auto-dismiss again)
3. Assess real-world impact — would this actually cause problems?
4. Consider the cost-benefit — is fixing this proportional to the risk?
5. Render a verdict with clear reasoning

## Output Format

For each finding:

**[Reviewer/Security] Finding N: [Title]**
- Source severity: [what the reviewer/auditor assigned]
- Verdict: VALID | DISMISSED
- Reasoning: [2-3 sentences explaining why, with specific reference to impact]

Overall verdict:

<!-- VERDICT -->
- REVIEWER_FINDING_1: VALID | DISMISSED
- REVIEWER_FINDING_2: VALID | DISMISSED
- SECURITY_FINDING_1: VALID | DISMISSED
- ...
- OVERALL: REVISE | APPROVE
- VALID_COUNT: [N]
- DISMISSED_COUNT: [N]
<!-- /VERDICT -->

OVERALL is APPROVE only if VALID_COUNT is 0. Any valid finding means REVISE.
```

### 5. Planner

Breaks the approved plan into executable tasks for the implementor fleet.

**Responsibilities:**
- Decompose the plan into discrete, independent tasks
- Identify dependencies between tasks (what blocks what)
- Maximize parallelism — tasks that can run concurrently should
- Assign each task a clear scope: which files to create/modify, what behavior to implement
- Recommend a branch strategy based on task relationships
- Ensure each task is self-contained enough for an agent to execute without context from other tasks

**Mode:** Non-interactive (prompt in, output out)
**Output:** `plan/task-breakdown.json` (includes dependency graph, merge order, and parallelism info)

**Base Prompt:**

```
You are a senior engineering manager who excels at breaking complex projects into
parallel workstreams. Your job is to take an approved implementation plan and decompose
it into discrete tasks that a fleet of coding agents can execute concurrently.

## Decomposition Principles

1. MAXIMIZE PARALLELISM
   - Tasks that touch different files or different subsystems should be independent
   - Only create dependencies when tasks truly cannot proceed without each other's output
   - Prefer more smaller tasks over fewer large tasks, as long as each task is coherent

2. CLEAR BOUNDARIES
   - Each task must have an unambiguous scope: exactly which files to create, modify, or delete
   - A task should never partially modify a file — if two tasks need the same file,
     either combine them or make one depend on the other
   - Each task must be completable without knowledge of what other tasks are doing

3. DEPENDENCY GRAPH
   - Explicitly state which tasks block which
   - Minimize the critical path — the longest chain of sequential dependencies
   - If a task has no dependencies, it runs immediately in parallel

4. SELF-CONTAINED TASK SPECS
   - Each task description must include everything an agent needs:
     - What to implement (specific behavior, not vague goals)
     - Which files to touch (exact paths)
     - Relevant context from the plan (don't make the agent re-read the full plan)
     - Expected outcome (what does "done" look like?)
     - Test expectations from the Test Architect (included by the orchestrator)
   - An agent reading only its task spec should be able to complete the work

5. CONFLICT AWARENESS
   - When tasks work in separate worktrees, flag files that might cause merge conflicts
   - Front-load tasks that touch shared infrastructure (types, configs, schemas)
   - Suggest merge order that minimizes conflict complexity

## Branch Strategy

Based on the dependency graph, recommend one of:
- **separate**: Each task gets its own branch. Best for independent tasks.
- **single**: All tasks work on one branch sequentially. Best for tightly coupled changes.
- **hybrid**: Independent clusters get separate branches, dependent chains share a branch.

## Output Format

Output valid JSON (no markdown wrapping) in this structure:

{
  "tasks": [
    {
      "id": "T1",
      "title": "Short descriptive title",
      "description": "Full task specification with all context needed",
      "files": {
        "create": ["path/to/new-file.ts"],
        "modify": ["path/to/existing-file.ts"],
        "delete": []
      },
      "depends_on": [],
      "estimated_complexity": "small | medium | large",
      "conflict_risk": ["path/to/shared-file.ts"]
    }
  ],
  "branch_strategy": "separate | single | hybrid",
  "merge_order": ["T1", "T2", "T3"],
  "critical_path": ["T1", "T3", "T5"],
  "parallelism_summary": "3 tasks parallel, then 1 sequential, then 2 parallel"
}
```

### 6. Test Architect

Designs the test strategy before implementation begins.

**Responsibilities:**
- Analyze the approved plan and the planner's task breakdown
- Define what tests each task should include (unit, integration, e2e)
- Identify edge cases and boundary conditions that tests must cover
- Specify integration tests needed after all tasks merge
- Produce test specs that get injected into each implementor's task prompt

**Mode:** Non-interactive (prompt in, output out)
**Output:** `plan/test-strategy.json`

**Base Prompt:**

```
You are a senior QA architect. Your job is to design a comprehensive test strategy
for a set of implementation tasks. You do not write tests — you specify what tests
need to exist so that coding agents include them during implementation.

## Your Approach

1. READ THE PLAN AND TASK BREAKDOWN
   Understand what's being built, how it's decomposed, and what each task does.

2. FOR EACH TASK, DEFINE TEST REQUIREMENTS
   - Unit tests: what functions/components need tests, what inputs and expected outputs
   - Integration tests: what interactions between components need verification
   - Edge case tests: boundary conditions, error scenarios, empty states, concurrent access

3. DEFINE POST-MERGE INTEGRATION TESTS
   After all tasks are merged, what tests verify the combined result works correctly?
   These are scenarios that only emerge when multiple tasks' outputs interact.

4. PRIORITIZE RUTHLESSLY
   Not everything needs a test. Focus on:
   - New behavior (every new function/endpoint gets tested)
   - Changed behavior (existing tests may need updates)
   - Error paths (what happens when things fail)
   - Security-sensitive paths (auth, input validation)
   Skip testing:
   - Trivial getters/setters
   - Framework boilerplate
   - Configurations

## Test Specification Format

Be specific. "Test the user flow" is useless. "Test that POST /api/themes returns 400
when theme name exceeds 50 characters" is useful.

Each test spec should include:
- Description of what to test
- Input/setup conditions
- Expected outcome
- Why this test matters (what bug would it catch?)

## Codebase Testing Patterns

Examine the existing test infrastructure before designing tests:
- What testing framework is used?
- What patterns do existing tests follow?
- What test utilities and helpers exist?
- What mocking patterns are established?

Your test specs must align with existing patterns. Do not introduce new testing
conventions.

## Output Format

Output valid JSON:

{
  "per_task": {
    "T1": {
      "unit_tests": [
        {
          "description": "What to test",
          "file": "path/to/test-file.test.ts",
          "setup": "Required setup/mocks",
          "expected": "Expected outcome",
          "rationale": "What bug this catches"
        }
      ],
      "integration_tests": [...],
      "edge_case_tests": [...]
    },
    "T2": { ... }
  },
  "post_merge": {
    "integration_tests": [
      {
        "description": "Cross-task integration scenario",
        "tasks_involved": ["T1", "T3"],
        "setup": "Required setup",
        "expected": "Expected outcome",
        "rationale": "Why this matters"
      }
    ]
  },
  "testing_patterns": {
    "framework": "detected framework",
    "conventions": "detected conventions to follow"
  }
}
```

### 7. Implementors

A fleet of coding agents that execute tasks in parallel.

**Responsibilities:**
- Execute the task spec: create, modify, and delete files as specified
- Write tests as specified by the Test Architect
- Run tests and ensure they pass
- Handle merge conflicts when working on dependent tasks (the implementor's worktree is pre-merged with dependency branches; if conflicts arise during their work, they resolve them inline)
- Stay strictly within the task's scope — do not make changes outside assigned files

**Mode:** Non-interactive, CLI subprocess (spawns `claude`/`codex`/`gemini` in yolo mode)
**Output:** Code changes in the task's worktree/branch, `implementation/task-<id>/status.json`

**Base Prompt (injected per task):**

```
You are a senior software developer executing a specific implementation task. You are
part of a fleet of agents working in parallel on different parts of the same project.

## Your Task

{task_spec from planner}

## Test Requirements

{test_spec from test architect}

## Rules

1. SCOPE
   - Only modify files listed in your task spec
   - Do not refactor, optimize, or "improve" code outside your scope
   - If you discover something that needs changing outside your scope, note it in your
     output but do not modify it

2. QUALITY
   - Follow existing codebase conventions exactly (naming, patterns, file structure)
   - Write clean, production-ready code — not prototypes or TODOs
   - Include error handling for failure modes identified in the plan

3. TESTING
   - Write every test specified in your test requirements
   - Run all tests and ensure they pass before finishing
   - If a test fails, fix the implementation — do not delete or weaken the test

4. CONFLICT AWARENESS
   - You are working in a git worktree. Other agents are working in parallel on other
     branches.
   - Your worktree may include merged changes from tasks you depend on.
   - If you encounter merge conflicts in your worktree, resolve them. Understand the
     intent of both sides from the plan context and make the correct merge decision.
   - If a conflict is ambiguous and you cannot confidently resolve it, stop and report
     the conflict in your output.

5. OUTPUT
   When finished, create a file called TASK_RESULT.md in the working directory root:

   ## Status: COMPLETE | FAILED | CONFLICT

   ## Changes Made
   - file1.ts: [what changed and why]
   - file2.ts: [what changed and why]

   ## Tests
   - X tests written, Y passing, Z failing
   - [list any failing tests with error details]

   ## Conflicts (if any)
   - [describe unresolvable conflicts]

   ## Out of Scope Observations
   - [anything you noticed that needs attention but is outside your task]
```

### 8. Validator

Verifies the implementation matches the plan and everything works together.

**Responsibilities:**
- Compare implemented code against the original approved plan
- Detect drift: missing functionality, extra changes, incomplete implementations
- Run the full test suite in the combined codebase
- Check for integration issues: import errors, type mismatches, circular dependencies
- If drift is found, spawn targeted implementor agents to fix specific issues
- Verify post-merge integration tests pass

**Mode:** Non-interactive (prompt in, output out), but can spawn implementor subprocesses
**Output:** `validation/report.md`, `validation/status.json`

**Base Prompt:**

```
You are a senior QA engineer performing final validation on a completed implementation.
Multiple coding agents have worked in parallel on different tasks. All their changes
have been merged into a single branch. Your job is to verify the result matches the
original plan and works correctly.

## Your Process

1. READ THE PLAN
   Understand what was supposed to be built. The plan is your source of truth.

2. READ EVERY TASK RESULT
   Each implementor produced a TASK_RESULT.md. Check:
   - Did each task complete successfully?
   - Were all specified files created/modified?
   - Were all tests written and passing?

3. CHECK FOR DRIFT
   Compare the actual codebase state against the plan:
   - Missing functionality: something in the plan that wasn't implemented
   - Extra changes: modifications outside the planned scope
   - Incomplete implementation: partially implemented features
   - Wrong approach: implementation that diverges from the plan's specified approach

4. RUN VALIDATION
   - Run the full test suite. Report any failures.
   - Run type checking / linting. Report any errors.
   - Check for import errors, missing exports, circular dependencies.
   - Run post-merge integration tests from the test strategy.

5. PRODUCE A REPORT
   For each drift or failure found:
   - What's wrong (specific file, function, behavior)
   - What the plan expected
   - What actually happened
   - How to fix it (specific enough for an implementor to act on)

## Drift Severity

- **BLOCKING**: Functionality is missing or broken. Must be fixed.
- **MINOR**: Cosmetic or non-functional drift. Can be flagged but doesn't need fixing.

## Output Format

<!-- VALIDATION -->
- STATUS: PASS | FAIL
- BLOCKING_ISSUES: [count]
- MINOR_ISSUES: [count]
- TEST_SUITE: [pass_count] passed, [fail_count] failed
- TYPE_CHECK: PASS | FAIL
<!-- /VALIDATION -->

If STATUS is FAIL, include a fixes section:

## Required Fixes

### Fix 1: [Title]
**Severity:** BLOCKING
**File:** path/to/file.ts
**Expected:** What the plan specified
**Actual:** What's there now
**Fix:** Specific instructions for the implementor
```

## State Machine

```
              RESEARCHING (interactive with user)
                    │
                    ▼
            [checkpoint?] ── user approves ──┐
                                             │
                    ┌────────────────────────┘
                    ▼
              ┌─ REVIEWING ──────┐
              │                  │  (parallel)
              └─ SECURITY AUDIT ─┘
                    │
                    ▼
                 JUDGING
                    │
               ┌────┴────┐
               │         │
            REVISE    APPROVE ───────────────────┐
               │                                 │
               ▼                                 │
    RESEARCHING (non-interactive revision)       │
               │                                 │
          max iterations? ── escalate to user    │
               │                                 │
          [checkpoint?]                          │
               │                                 │
               └── back to REVIEWING             │
                                                 │
                    ┌────────────────────────────┘
                    ▼
                PLANNING
                    │
                    ▼
            TEST ARCHITECTING
                    │
                    ▼
            [checkpoint?] ── user approves ──┐
                                             │
                    ┌────────────────────────┘
                    ▼
             IMPLEMENTING
              (parallel fleet)
                    │
                    ▼
              VALIDATING
                    │
               ┌────┴────┐
               │         │
             FAIL      PASS ─── [merge flow] ─── COMPLETE
               │
               ▼
         FIXING (spawns implementors)
               │
          max iterations? ── escalate to user
               │
               └── back to VALIDATING
```

**Transition table:**

| From | To | Condition |
|------|----|-----------|
| Researching | AwaitingApproval(Reviewing) | checkpoint configured |
| Researching | Reviewing | no checkpoint |
| Reviewing + SecurityAuditing | Judging | both complete |
| Judging | Researching (revision) | REVISE verdict |
| Judging | Planning | APPROVE verdict |
| Researching (revision) | Reviewing | revision complete |
| Planning | TestArchitecting | always |
| TestArchitecting | AwaitingApproval(Implementing) | checkpoint configured |
| TestArchitecting | Implementing | no checkpoint |
| Implementing | Validating | all tasks complete |
| Validating | Fixing | FAIL with blocking issues |
| Validating | Complete | PASS |
| Fixing | Validating | fixes applied |

The `[checkpoint?]` gates are configurable. By default, checkpoints are placed after
researching and after planning. The user can add or remove checkpoints via
`kora configure` or config file.

The `AwaitingApproval` state wraps the next stage:

```rust
enum Stage {
    Researching,
    Reviewing,
    SecurityAuditing,
    Judging,
    Planning,
    TestArchitecting,
    Implementing,
    Validating,
    Fixing,
    AwaitingApproval(Box<Stage>),
    Complete,
    Failed(String),
}
```

## Provider Abstraction

All AI interaction goes through CLI subprocess spawning. No API keys, no direct API calls.

```rust
trait Provider: Send + Sync {
    fn name(&self) -> &str;

    fn is_available(&self) -> Result<bool>;

    async fn run(
        &self,
        prompt: &str,
        working_dir: &Path,
        config: &AgentConfig,
        output: &mut TerminalOutput,
    ) -> Result<AgentOutput>;

    async fn run_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
        config: &AgentConfig,
    ) -> Result<InteractiveSession>;
}
```

Each provider adapter knows how to invoke its CLI in fully autonomous mode:

| Provider | CLI Command | Autonomous Flag |
|----------|------------|-----------------|
| Claude | `claude` | `--dangerously-skip-permissions` |
| Codex | `codex` | `--approval-mode full-auto` |
| Gemini | `gemini` | TBD (added when supported) |

The orchestrator always passes the autonomous flag. Underlying CLIs never prompt the user.

## Agent Communication

Agents do not communicate directly. The orchestrator mediates all communication through
files in the run directory.

```
Orchestrator reads researcher-plan.md
    → injects into reviewer prompt
    → injects into security auditor prompt

Orchestrator reads review.md + security-audit.md
    → injects into judge prompt

Orchestrator reads judgment.md
    → if REVISE: injects valid findings into researcher revision prompt (non-interactive)
    → researcher produces revised plan
    → BOTH reviewer and security auditor re-run on the revised plan (next iteration)
    → if APPROVE: injects plan into planner prompt

Orchestrator reads task-breakdown.json + researcher-plan.md + codebase-summary.md
    → injects into test architect prompt
    → test architect has filesystem access to examine existing test patterns

Orchestrator reads task-breakdown.json + test-strategy.json
    → constructs per-task prompts for implementors

Orchestrator reads TASK_RESULT.md from each implementor
    → copies TASK_RESULT.md from worktree root to implementation/task-<id>/
    → merges branches
    → injects everything into validator prompt
```

Each agent is stateless. A Claude researcher can hand off to a Codex reviewer because
the handoff is a file, not a conversation thread.

## Structured Output Parsing

Agents include structured markers in their output that the orchestrator parses:

```
<!-- VERDICT -->
- REVIEWER_FINDING_1: VALID
- SECURITY_FINDING_1: DISMISSED
- OVERALL: REVISE
<!-- /VERDICT -->
```

The orchestrator scans for these markers using simple string matching.

**Failure handling:**
1. If markers are missing, re-prompt the agent once: "Include the structured output
   section as specified."
2. If markers are malformed, attempt lenient parsing (case-insensitive, whitespace-tolerant).
3. If parsing still fails, save the raw output and pause for human intervention.

## Terminal UX

### Visual Language

The terminal experience is modeled after Claude Code: streaming output, minimal chrome,
conversational interaction. The orchestrator renders directly to the terminal using
crossterm for control and termimad for markdown.

**Stage headers** use dot-leaders and a live spinner:

```
  researcher ·········································· starting ●
```

**Severity glyphs** provide instant visual weight without color dependency:

```
  ▲ HIGH    ■ MED    · LOW
```

**Findings use indented blocks:**

```
    ▲ HIGH   No database migration strategy for the existing
             user_preferences table. Existing rows would have
             null theme values after deploy.
```

**Verdicts are inline:**

```
    ▲ DB migration          accepted — high impact, would break prod
    · Const enum            dismissed — stylistic, no functional impact
```

**Implementation dashboard updates in-place:**

```
  implementing ······································ 2 of 4 ●

    T1  claude  ████████░░░░  running   feat/theme-context
    T2  codex   ████████████  ✓ 12s     feat/css-variables    3 files
    T3  claude  █████░░░░░░░  running   feat/migration
    T4  claude  ○○○○○○○○○○○○  blocked   feat/integration     → T1,T3
```

**Users can drill into running tasks:**

```
  > show T1

  T1 · feat/theme-context · claude ·················· running ●

  [live streaming output from claude CLI]
  ...

  > back
```

### Verbosity Modes

Three levels, toggled with Tab:

| Mode | What You See |
|------|-------------|
| **focused** (default) | Stage transitions, verdicts, approval prompts |
| **detailed** | + findings, revision summaries, task output |
| **verbose** | + full agent streaming output |

Tab cycles: focused → detailed → verbose → focused.

A small mode indicator shows in the corner. On first run, a hint appears:
`press Tab for more detail`.

Approval prompts always render in full regardless of verbosity mode.

Default verbosity is configurable:

```yaml
output:
  default_verbosity: focused
```

### Selection Prompts

All choices use arrow-key navigation:

```
  ? What would you like to do with the changes?

    ❯ Merge all into current branch
      Create a PR combining all changes
      Create separate PRs per task
      Leave branches as-is

                                          ↑↓ navigate · enter select
```

Multi-select uses space to toggle:

```
  ? Which runs to clean?

    ◉ "add dark mode support"           12MB · today
    ◯ "fix N+1 query"                    4MB · today
    ◉ "add RBAC to billing endpoints"    8MB · yesterday

                              ↑↓ navigate · space toggle · enter confirm
```

## CLI Commands

```
kora                    interactive session (primary entry point)
kora configure          setup wizard
kora run "<request>"    one-shot run, skip the chat
kora resume             resume interrupted session
kora history            past runs, human-readable
kora clean              cleanup old run data
```

### `kora` (default)

Drops into an interactive session:

```
$ kora

  kora v0.1.0 · claude (default) · 2 checkpoints configured

  ready. describe what you'd like to build, fix, or change.

> add dark mode support that respects system preferences

  researcher ·········································· starting ●
  ...
```

The session stays alive after runs complete. Multiple runs per session.

Inline meta commands:

```
> /status            current/recent runs
> /config            quick config view
> /verbose           toggle verbosity
> /help              list commands
```

### `kora run "<request>"`

One-shot mode for automation or when the user knows exactly what they want:

```
$ kora run "fix the N+1 query in deployments" --yolo
```

Flags:

| Flag | Effect |
|------|--------|
| `--careful` | Checkpoints at every stage |
| `--yolo` | No checkpoints, full autopilot |
| `--dry-run` | Research + review loop only, no implementation |
| `-p <provider>` | Override default provider for this run |

### `kora resume`

Finds interrupted sessions and lets the user pick:

```
$ kora resume

  "add dark mode support" · implementing · 2 of 4 tasks done · 4 min ago

  resuming...
```

Multiple interrupted sessions use an arrow-key selector.

### `kora history`

Human-readable past runs with drill-down:

```
$ kora history

  today
    ✓ "add dark mode support"          4 tasks · 3m 12s
    ✓ "fix N+1 query in deployments"   1 task  · 45s

  yesterday
    ✓ "add RBAC to billing endpoints"  6 tasks · 5m 03s
    ✗ "migrate to new auth provider"   failed at review · 2m 11s
```

Arrow keys to select a run and view details.

### `kora clean`

Interactive cleanup:

```
$ kora clean

  3 completed runs using 24MB

  ? Clean up:
    ❯ All completed runs
      Older than 1 week
      Pick specific ones
```

### `kora configure`

Interactive setup wizard:

```
$ kora configure

  ? Default AI provider:
    ❯ claude
      codex

  ? Assign providers per agent role, or use default for all?
    ❯ Use default for all
      Assign per role

  ? Which stages require your approval before proceeding?
    ◉ After researcher proposes direction
    ◯ After each review/judge iteration
    ◉ After planner breaks down tasks
    ◯ After implementation completes

  ? Maximum review loop iterations before escalating: (3)

  ? Default branch strategy for implementation:
    ❯ Separate branch per task
      Single feature branch
      Let planner decide

  ? Default output verbosity:
    ❯ focused
      detailed
      verbose

  ✅ Configuration saved to .kora/config.yml
```

## Configuration

### Config File

Located at `.kora/config.yml` in the repo root. Committed to version control so
the team shares orchestration settings. `.kora/runs/` is added to `.gitignore`.

```yaml
version: 1

default_provider: claude

providers:
  claude:
    cli: claude
    flags: []
  codex:
    cli: codex
    flags: []

agents:
  researcher:
    provider: default
    custom_instructions: null     # path to .md file with extra instructions
  reviewer:
    provider: default
    custom_instructions: null
  security_auditor:
    provider: default
    custom_instructions: null
  judge:
    provider: default
    custom_instructions: null
  planner:
    provider: default
    custom_instructions: null
  test_architect:
    provider: default
    custom_instructions: null
  implementor:
    provider: default
    custom_instructions: null
  validator:
    provider: default
    custom_instructions: null

checkpoints:
  - after_researcher
  - after_planner

review_loop:
  max_iterations: 3

validation_loop:
  max_iterations: 2

implementation:
  branch_strategy: separate
  parallel_limit: 4

output:
  default_verbosity: focused

runs_dir: .kora/runs
```

### Custom Instructions

Users can layer additional instructions on top of any agent's base prompt by pointing
to a markdown file:

```yaml
agents:
  researcher:
    custom_instructions: .kora/prompts/researcher-extra.md
  reviewer:
    custom_instructions: .kora/prompts/reviewer-extra.md
```

The file contents are appended to the base prompt:

```
[base prompt baked into binary]

---

## Additional Instructions

[contents of custom_instructions file]
```

The base prompt is never replaceable — only extendable.

## Run Directory Structure

```
.kora/
  config.yml
  .gitignore                      ← ignores runs/
  runs/
    <run-id>/
      state.json                  ← current stage, timestamps, config snapshot
      request.md                  ← original user request + clarifications
      context/
        codebase-summary.md       ← researcher's codebase analysis
        researcher-plan.md        ← researcher's plan (latest revision)
      reviews/
        iteration-1/
          review.md               ← reviewer findings
          security-audit.md       ← security auditor findings
          judgment.md             ← judge verdict
        iteration-2/
          ...
      plan/
        task-breakdown.json       ← planner output (tasks, dependencies, merge order)
        test-strategy.json        ← test architect output
      implementation/
        task-<id>/
          branch.txt              ← branch name
          status.json             ← pending/running/done/failed
          log.md                  ← agent output log
          TASK_RESULT.md          ← implementor's completion report
      validation/
        report.md                 ← validator findings
        status.json               ← pass/fail
        fixes/
          fix-<n>/                ← spawned fixer task data
            ...
```

## Implementor Failure Handling

When an implementor fails:

1. **Retry once** — fresh worktree, same prompt with the error output appended:
   "The previous attempt failed with the following error. Fix the issue and complete
   the task."
2. **Try a different provider** — if multiple providers are configured, retry with the
   next available provider.
3. **Escalate to user** — display the failure context and ask:
   ```
   T3 failed after 2 attempts:
     attempt 1 (claude): type error in migration script
     attempt 2 (codex):  test timeout in integration test

   ? How to proceed?
     ❯ Retry with specific instructions
       Skip this task
       Abort the run
   ```

## Task Dependency Handoff

When a blocked task becomes unblocked:

1. The orchestrator creates a new worktree from the base branch.
2. It merges all completed dependency branches into the worktree.
3. If the merge succeeds cleanly, the implementor starts.
4. If the merge has conflicts, the orchestrator spawns the implementor with conflict
   resolution context: both sides of the conflict + the relevant plan sections for
   each task. The implementor resolves conflicts as its first action.

## Merge Conflict Resolution

After all tasks complete and during the final merge flow:

1. The orchestrator merges branches in the order specified by the planner.
2. If a conflict occurs, the implementor agent responsible for the conflicting task
   is respawned with:
   - Both sides of the conflict
   - The plan context for the relevant tasks
   - Instructions to resolve the conflict and verify tests still pass
3. After resolution, the merge continues with the next branch.

## Prompt Delivery to CLI Agents

Each provider has a specific mechanism for receiving prompts:

**Claude:**
```bash
# Non-interactive: --print flag outputs result and exits, -p passes the prompt
claude --print --dangerously-skip-permissions -p "prompt text here"

# For long prompts: pipe via stdin
echo "prompt text here" | claude --print --dangerously-skip-permissions

# Interactive (researcher only): spawn with initial prompt, user interacts directly
claude --dangerously-skip-permissions -p "initial prompt"
```

**Codex:**
```bash
# Non-interactive: --quiet suppresses interactive UI
codex --approval-mode full-auto --quiet "prompt text here"

# Interactive (researcher only):
codex --approval-mode full-auto "initial prompt"
```

For prompts exceeding shell argument limits (~100KB), the orchestrator writes the prompt
to a temporary file and pipes it via stdin. The temp file is deleted after the agent
exits.

The orchestrator captures agent output by reading stdout. For non-interactive agents,
the full stdout is captured and written to the run directory. For interactive agents
(researcher only), stdout is tee'd to both the terminal and a log file.

## Interactive Session Lifecycle (Researcher)

The researcher is the only agent that runs interactively. Its lifecycle:

1. **Initial session:** The orchestrator spawns the CLI agent with the researcher's
   base prompt + codebase context + user request. stdin/stdout are piped directly
   between the user's terminal and the agent. The orchestrator tees stdout to a log file.

2. **Session completion:** The researcher's prompt instructs it to write its final plan
   to a specific file path (`context/researcher-plan.md` in the working directory)
   when it considers the plan ready. The orchestrator watches for this file using
   filesystem polling (100ms interval). When the file appears, the orchestrator waits
   for the CLI process to exit, then reads the plan file and advances the state machine.

3. **Fallback detection:** If the CLI process exits without creating the plan file,
   the orchestrator extracts the plan from stdout using the structured markers
   (`<!-- PLAN -->...<!-- /PLAN -->`), saves it to the expected path, and proceeds.
   If neither the file nor markers exist, it prompts the user: "The researcher didn't
   produce a structured plan. Would you like to retry or provide the plan manually?"

4. **Revision mode:** When the judge sends findings back, the researcher runs in
   non-interactive mode. The orchestrator constructs a revision prompt containing the
   current plan + valid findings + judge reasoning, and runs it via `provider.run()`
   (not `run_interactive()`). The revised plan is captured from the output. This is
   non-interactive because the user already approved the direction — the revision is
   mechanical ("fix these specific issues"), not exploratory. If a checkpoint is
   configured after researcher revisions, the orchestrator displays the revised plan
   and prompts for approval before continuing the review loop.

## Non-Interactive Agent Failure Handling

When a non-interactive agent (reviewer, security auditor, judge, planner, test
architect, validator) fails:

1. **CLI crash / non-zero exit:** Retry once with the same prompt. If it fails again,
   pause and show the user:
   ```
   reviewer crashed (exit code 1):
     [last 10 lines of stderr]

   ? How to proceed?
     ❯ Retry
       Retry with different provider
       Skip this stage
       Abort the run
   ```

   **"Skip this stage" semantics by role:**
   - **Reviewer:** skip review, security auditor findings only go to judge
   - **Security Auditor:** skip security audit, reviewer findings only go to judge
   - **Judge:** skip judgment, treat all findings as DISMISSED and advance to planner
   - **Planner:** cannot skip (no tasks = nothing to implement). Option not shown.
   - **Test Architect:** skip test strategy, implementors work without test specs
   - **Validator:** skip validation, proceed directly to merge flow

2. **Timeout:** Non-interactive agents have a configurable timeout (default: 5 minutes).
   If exceeded, the process is killed and treated as a crash (follows step 1).

3. **Empty output:** Treated as a crash (follows step 1).

4. **Malformed structured output:** Handled by the structured output parsing rules
   (re-prompt once, then lenient parsing, then escalate to user).

Timeout is configurable:
```yaml
agents:
  reviewer:
    timeout_seconds: 300
```

## Validation Loop Escalation

When the validation loop exceeds `max_iterations`:

```
  validator ·········································· iteration 3 ●

  Still 2 blocking issues after 3 fix attempts:

    ▲ Type mismatch in theme-context.tsx:42 — ThemeValue vs string
    ▲ Integration test failing: "theme persists across page reload"

  ? How to proceed?
    ❯ Retry fixes with specific instructions
      Open an interactive session to fix manually
      Accept current state (merge with known issues)
      Abort the run
```

"Open an interactive session" spawns the default CLI agent in the merged worktree,
giving the user full control to fix the remaining issues manually with AI assistance.

## Rust Crate Stack

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime for concurrent agent management |
| `crossterm` | Terminal control, raw mode, colors, cursor positioning |
| `termimad` | Markdown rendering in terminal |
| `indicatif` | Spinners and progress bars for implementation dashboard |
| `serde` + `serde_json` + `serde_yaml` | State serialization, config parsing |
| `clap` | CLI argument parsing |
| `uuid` | Run ID generation (internal only, never shown to user) |
| `chrono` | Timestamps for run tracking |
| `walkdir` | Codebase file tree analysis |
| `which` | Provider CLI detection (is `claude` installed?) |

## Distribution

Single binary, installable via:

```bash
# npm (most users already have Node.js)
npm install -g kora-ai

# Homebrew (macOS/Linux)
brew install kora-ai/tap/kora

# Cargo (Rust toolchain)
cargo install kora

# Direct download (GitHub releases)
curl -fsSL https://raw.githubusercontent.com/kora-ai/kora/main/install.sh | sh
```

The npm package downloads the correct pre-built binary for the user's platform
during `postinstall` — no Rust toolchain required.

No runtime dependencies beyond the user having at least one supported CLI agent
installed (`claude`, `codex`, or `gemini`).
