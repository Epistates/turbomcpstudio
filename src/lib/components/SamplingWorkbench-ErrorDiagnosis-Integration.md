# Integration Plan: Error Diagnosis in SamplingWorkbench

## Current State Analysis

### SamplingRequest Interface
```typescript
interface SamplingRequest {
  id: string;
  serverId: string;
  serverName: string;
  messages: SamplingMessage[];
  modelPreferences?: ModelPreferences;
  systemPrompt?: string;
  includeContext?: 'none' | 'thisServer' | 'allServers';
  maxTokens?: number;
  temperature?: number;
  stopSequences?: string[];
  timestamp: string;
  status: 'pending' | 'approved' | 'rejected' | 'completed' | 'error';
  response?: SamplingResponse;
  duration?: number;
  cost?: number;
}
```

**Missing**: `error?: any` field to store error information

## Required Changes

### 1. Update SamplingRequest Interface

Add error field:
```typescript
interface SamplingRequest {
  // ... existing fields
  status: 'pending' | 'approved' | 'rejected' | 'completed' | 'error';
  response?: SamplingResponse;
  error?: any;  // ← ADD THIS
  duration?: number;
  cost?: number;
}
```

### 2. Update Error Handling in approveAndProcess

Around line 700-800 where errors are caught:

```typescript
async function approveAndProcess(request: SamplingRequest, useAI: boolean) {
  try {
    // ... existing code
  } catch (error) {
    console.error('Sampling request failed:', error);

    // Store error information
    request.status = 'error';
    request.error = error;  // ← ADD THIS

    samplingRequests = [...samplingRequests];

    uiStore.showError(`Sampling failed: ${error}`);
  } finally {
    processing = false;
  }
}
```

### 3. Add Error Diagnosis Display in Response Panel

Around line 1650-1680, add error handling:

```svelte
{:else if selectedRequest.status === 'error'}
  <!-- Replace the simple error message with ErrorDiagnosis component -->
  <div class="p-4">
    {@const diagnosis = diagnoseError(selectedRequest.error, {
      estimatedTokens: estimatedCost ? Math.ceil(estimatedCost * 1000) : undefined,
      maxContextWindow: maxContextWindow,
      serverStatus: servers.find(s => s.id === selectedRequest.serverId)?.status,
      hasApiKey: !!(activeProvider?.configured)
    })}

    <ErrorDiagnosis
      {diagnosis}
      onRetry={() => {
        // Copy request params to test message
        testMessage = formatMessageContent(selectedRequest.messages.find(m => m.role === 'user')?.content || '');
        testSystemPrompt = selectedRequest.systemPrompt || '';
        testMaxTokens = selectedRequest.maxTokens || 500;
        testTemperature = selectedRequest.temperature || 0.7;

        // Create new request
        createTestSamplingRequest();
      }}
    />
  </div>
{:else if selectedRequest.status === 'pending'}
  <!-- existing pending UI -->
```

### 4. Add Imports

At the top of SamplingWorkbench.svelte:

```typescript
import ErrorDiagnosis from './ui/ErrorDiagnosis.svelte';
import { diagnoseError } from '$lib/utils/errorDiagnosis';
```

### 5. Update createTestSamplingRequest Error Handling

Around line 600-750 where test requests are created:

```typescript
async function createTestSamplingRequest() {
  try {
    // ... create request
  } catch (error) {
    console.error('Failed to create sampling request:', error);

    // Show diagnosis immediately
    const diagnosis = diagnoseError(error, {
      estimatedTokens: estimatedCost ? Math.ceil(estimatedCost * 1000) : undefined,
      maxContextWindow: maxContextWindow,
      hasApiKey: !!(activeProvider?.configured)
    });

    uiStore.showError(diagnosis.diagnosis);
  } finally {
    creatingRequest = false;
  }
}
```

## Implementation Steps

1. ✅ Created errorDiagnosis.ts utility
2. ✅ Created ErrorDiagnosis.svelte component
3. ⏳ Update SamplingRequest interface
4. ⏳ Integrate into SamplingWorkbench error handling
5. ⏳ Test with various error scenarios

## Testing Scenarios

1. **Server not connected**: Disconnect server, try to send request
2. **Invalid API key**: Use wrong API key
3. **Context window exceeded**: Send very long message
4. **Network error**: Disconnect internet, try request
5. **Rate limit**: Make many requests quickly
6. **Validation error**: Send malformed request

Each should show helpful diagnosis with actionable suggestions.
