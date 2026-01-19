# CALL_AGENT Investigation

**Date**: 2026-01-19
**Status**: Issue persists despite multiple fix attempts
**Priority**: High - blocks A2A capability showcase

## Issue Summary

TestCoordinator agent (ID 51) generates CALL_AGENT actions that are rejected by the handler with error:
```
"Please specify an agent to call (e.g., 'call agent compute.jeju')"
```

Despite explicit character prompts instructing the LLM to include the exact phrase `call agent http://localhost:4021/a2a skill echo`, the handler consistently rejects the calls.

## Handler Implementation Analysis

### Location
`/Users/hellno/dev/misc/jeju/packages/eliza-plugin/src/actions/a2a.ts`

### Parsing Mechanism (lines 54-72)
```typescript
const agentMatch = text.match(/agent\s+([^\s]+)/i)
const skillMatch = text.match(/skill\s+([^\s]+)/i)

if (!agentMatch) {
  callback?.({
    text: "Please specify an agent to call (e.g., 'call agent compute.jeju')",
  })
  return
}

if (!skillMatch) {
  callback?.({
    text: "Please specify a skill to call (e.g., 'call agent compute.jeju skill list-providers')",
  })
  return
}

const agentEndpoint = agentMatch[1]
const skillId = skillMatch[1]

callback?.({ text: `Calling agent ${agentEndpoint} skill ${skillId}...` })

const response = await client.a2a.callSkill(agentEndpoint, skillId, {
  message: text,
})
```

**Key Points**:
- Handler parses natural language TEXT using regex, NOT structured JSON parameters
- Regex: `/agent\s+([^\s]+)/i` captures first non-whitespace token after "agent"
- Regex: `/skill\s+([^\s]+)/i` captures first non-whitespace token after "skill"
- Extraction is case-insensitive
- No structured parameter passing

## SDK Auto-Append Discovery

### Location
`/Users/hellno/dev/misc/jeju/packages/sdk/src/a2a/index.ts:221`

### Code
```typescript
const a2aUrl = endpoint.endsWith('/a2a') ? endpoint : `${endpoint}/a2a`
const response = await fetch(a2aUrl, {
  method: 'POST',
  headers,
  body: JSON.stringify(body),
})
```

**Critical Finding**: The SDK automatically appends `/a2a` to the endpoint if it doesn't already end with `/a2a`.

**Implication**: Even if the LLM generates `http://localhost:4021`, the SDK converts it to `http://localhost:4021/a2a` before making the HTTP request. **URL shortening is NOT the actual problem.**

## Test Results

### Latest Test (Agent 51, after character fixes)
```json
{
  "action": "CALL_AGENT",
  "timestamp": 1768837076495,
  "success": true,
  "result": {
    "executed": true,
    "params": {
      "agent": "http://localhost:4021",
      "skill": "echo"
    },
    "result": {
      "response": "Please specify an agent to call (e.g., 'call agent compute.jeju')"
    }
  }
}
```

**Observations**:
- Action shows `success: true` and `executed: true`
- Params extracted: `agent: "http://localhost:4021"`, `skill: "echo"`
- Handler returned rejection message
- Despite SDK auto-appending `/a2a`, handler still rejects

## Character Prompt Evolution

### Initial Approach
Used JSON-style format in system prompt (didn't work - handler expects natural language):
```
{agent: "http://localhost:4021/a2a", skill: "echo"}
```

### Second Attempt
Changed to natural language with embedded keywords:
```
"I will call agent http://localhost:4021/a2a skill echo to demonstrate A2A communication"
```
**Result**: LLM shortened URL to `http://localhost:4021`

### Third Attempt (Current)
Added CRITICAL FORMAT REQUIREMENTS section explicitly stating:
- The URL MUST include /a2a at the end
- DO NOT shorten the URL
- DO NOT use JSON format
- Example showing exact phrase

**Result**: Still drops `/a2a` from URL in generated text

### Fourth Attempt
Added style directives:
```typescript
style: {
  all: [
    'ALWAYS include the exact phrase: call agent http://localhost:4021/a2a skill echo',
    'DO NOT modify or shorten this URL under any circumstances',
    'DO NOT remove /a2a from the endpoint',
  ]
}
```

**Result**: LLM still shortens URL

## Theories About Root Cause

### Theory 1: Regex Not Matching (UNLIKELY)
The regex `/agent\s+([^\s]+)/i` should match `call agent http://localhost:4021` and extract `http://localhost:4021`.

Test in Node REPL:
```javascript
const text = "I will call agent http://localhost:4021 skill echo"
const agentMatch = text.match(/agent\s+([^\s]+)/i)
// agentMatch[1] === "http://localhost:4021" ✓
```

Regex should work. This theory is unlikely.

### Theory 2: Text Not Being Passed to Handler (POSSIBLE)
The handler receives `text` parameter from eliza-plugin action invocation. Need to verify:
- Is the LLM-generated message text being passed correctly?
- Is there middleware transforming the text before handler sees it?
- Does the action extraction process modify the original text?

### Theory 3: Handler Code Path Not Being Reached (POSSIBLE)
Handler returns "Please specify an agent" when `!agentMatch`. Possible causes:
- Text is empty or malformed when handler receives it
- Text doesn't contain the expected keywords
- Handler is receiving a different text format than expected

### Theory 4: A2A Endpoint Rejection (POSSIBLE BUT DIFFERENT ISSUE)
Handler successfully extracts params and calls `client.a2a.callSkill()`, but the A2A endpoint itself rejects the call. However, the error message "Please specify an agent" comes from the handler's validation, not from the A2A endpoint response.

### Theory 5: LLM Output Format Issue (LIKELY)
The params show extracted values: `{agent: "http://localhost:4021", skill: "echo"}`. But the handler returns validation error. This suggests:
- The extraction MIGHT be working (params are populated)
- But the handler validation STILL fails (!agentMatch returns true)
- **Contradiction**: How can params be extracted if agentMatch is false?

**Possible explanation**: The `params` shown in test results might be from a DIFFERENT code path (autonomous action execution) that extracts structured data, while the handler still processes the raw text and fails validation.

## Critical Unanswered Questions

1. **What is the ACTUAL text being passed to the a2a.ts handler?**
   - Need to add debug logging in handler to see exact text received

2. **Where do the `params` in the test result come from?**
   - Are they extracted by autonomous/index.ts action execution?
   - Or by the eliza-plugin handler?
   - If different code paths, they might not align

3. **Why does the handler return validation error if params are extracted?**
   - Either params are from different source, OR
   - Handler validation runs before extraction

4. **What does the LLM actually generate in the message?**
   - Need to log the full LLM response text before action parsing

## Next Debugging Steps

### Step 1: Add Debug Logging to Handler
```typescript
// In packages/eliza-plugin/src/actions/a2a.ts
export const a2aAction: Action = {
  name: 'CALL_AGENT',
  handler: async (runtime, message, state, options, callback) => {
    const text = message.content.text

    // ADD THIS
    console.log('[A2A_HANDLER] Received text:', text)

    const agentMatch = text.match(/agent\s+([^\s]+)/i)
    const skillMatch = text.match(/skill\s+([^\s]+)/i)

    // ADD THIS
    console.log('[A2A_HANDLER] Regex matches:', { agentMatch, skillMatch })

    if (!agentMatch) {
      callback?.({
        text: "Please specify an agent to call (e.g., 'call agent compute.jeju')",
      })
      return
    }
    // ...
  }
}
```

### Step 2: Log Full LLM Response
Add logging in autonomous/index.ts where LLM response is received (around line 1742):
```typescript
const response = await runtime.executeAction(availableAction, {
  agentId: agent.id.toString(),
  characterName: agent.agentCharacter?.name || agent.name,
  description: action.context,
  currentRound: context.currentRound,
})

// ADD THIS
log.debug('[LLM_RESPONSE] Full response text:', response.text)
log.debug('[LLM_RESPONSE] Actions in response:', response.actions)
```

### Step 3: Trace Action Extraction
In autonomous/index.ts, add logging where actions are extracted from LLM response:
```typescript
// Around line 1750-1760
const actions = response.actions || []
log.debug('[ACTION_EXTRACTION] Extracted actions:', {
  count: actions.length,
  actions: actions.map(a => ({
    type: a.type,
    text: a.text, // ADD THIS - see what text is in the action
    params: a.params,
  }))
})
```

### Step 4: Compare Text Paths
Verify if there are TWO different code paths:
1. Autonomous action executor extracting structured params
2. Eliza-plugin handler receiving raw text

If they diverge, that explains the contradiction.

### Step 5: Test with Minimal Phrase
Update character to use simplest possible phrase:
```
"call agent localhost skill echo"
```

If this works, then the issue is URL format complexity, not fundamental parsing.

### Step 6: Review Action Type Registration
Check if CALL_AGENT action is registered in both:
- eliza-plugin action registry
- autonomous action type mapping

Mismatch could cause wrong handler to be invoked.

## Working Hypothesis

Based on the contradiction between extracted params and validation error, **most likely explanation**:

The `params` in test results are extracted by autonomous/index.ts using a DIFFERENT parsing logic (possibly looking for structured JSON or action annotations), while the actual eliza-plugin handler receives raw LLM text that doesn't match the expected format.

**If true**, the fix would be to ensure the LLM generates text in the EXACT format the handler expects, which may be different from what we've been instructing.

**Alternative**: The handler might not be receiving the text at all - the action might be intercepted by autonomous executor which extracts params but then fails to properly invoke the handler.

## Files Modified During Investigation

1. `/Users/hellno/dev/misc/jeju/apps/crucible/api/characters/test-coordinator.ts`
   - Added CRITICAL FORMAT REQUIREMENTS
   - Updated system prompt 3 times
   - Added style directives
   - All changes committed

2. Test Results Reviewed
   - `/tmp/claude/-Users-hellno-dev-misc-jeju-apps-crucible/tasks/*.output`
   - Agent 51 trajectory showing CALL_AGENT rejection

## Related Issues

- **UPLOAD_FILE**: Fixed by removing placeholder from JSON format
- **RUN_INFERENCE**: Fully working end-to-end
- **DWS Storage**: Deferred (separate backend issue)

## Status

**BLOCKED** pending deeper investigation of:
1. Actual text passed to handler
2. Action extraction vs handler invocation flow
3. Potential dual code paths for action execution

**Recommendation**: Add comprehensive debug logging before attempting more character prompt changes. The issue is likely architectural (how actions flow from LLM → autonomous executor → handler), not prompt engineering.
