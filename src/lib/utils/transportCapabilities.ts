/**
 * Transport Capability Reference
 *
 * Comprehensive information about MCP transport capabilities.
 * ALL transports support ALL MCP features when properly implemented!
 *
 * Based on MCP Specification 2025-06-18
 */

export interface TransportInfo {
  /** Display name for UI */
  name: string;
  /** Whether transport supports bidirectional communication (server → client requests) */
  bidirectional: boolean;
  /** Whether transport supports sampling (server → client LLM requests) */
  supportsSampling: boolean;
  /** Whether transport supports elicitation (server → client user input requests) */
  supportsElicitation: boolean;
  /** Implementation complexity level */
  complexity: 'low' | 'medium' | 'high';
  /** Typical use case description */
  typicalUseCase: string;
  /** MCP spec status */
  specStatus: 'standard' | 'custom';
  /** Brief description of how bidirectional works */
  bidirectionalMechanism?: string;
}

/**
 * Transport capability reference data
 *
 * KEY INSIGHT: ALL transports support ALL MCP features!
 * Differences are in connection model and use cases, not capabilities.
 */
export const TRANSPORT_CAPABILITIES: Record<string, TransportInfo> = {
  stdio: {
    name: 'STDIO',
    bidirectional: true,
    supportsSampling: true,
    supportsElicitation: true,
    complexity: 'low',
    typicalUseCase: 'CLI tools and local processes',
    specStatus: 'standard',
    bidirectionalMechanism: 'Native bidirectional via stdin/stdout pipes'
  },
  http: {
    name: 'HTTP/SSE',
    bidirectional: true,  // ✅ Via Server-Sent Events (SSE)
    supportsSampling: true,
    supportsElicitation: true,
    complexity: 'medium',
    typicalUseCase: 'Web services and cloud APIs',
    specStatus: 'standard',
    bidirectionalMechanism: 'Server-Sent Events (SSE) for server → client messages'
  },
  websocket: {
    name: 'WebSocket',
    bidirectional: true,
    supportsSampling: true,
    supportsElicitation: true,
    complexity: 'low',
    typicalUseCase: 'Real-time web applications',
    specStatus: 'custom',
    bidirectionalMechanism: 'Native WebSocket full-duplex communication'
  },
  tcp: {
    name: 'TCP',
    bidirectional: true,
    supportsSampling: true,
    supportsElicitation: true,
    complexity: 'low',
    typicalUseCase: 'Network services and microservices',
    specStatus: 'custom',
    bidirectionalMechanism: 'Native TCP socket bidirectional I/O'
  },
  unix: {
    name: 'Unix Socket',
    bidirectional: true,
    supportsSampling: true,
    supportsElicitation: true,
    complexity: 'low',
    typicalUseCase: 'Local IPC on macOS/Linux',
    specStatus: 'custom',
    bidirectionalMechanism: 'Unix domain socket bidirectional I/O'
  }
};

/**
 * MCP Feature Types
 */
export type MCPFeature =
  | 'tools'
  | 'prompts'
  | 'resources'
  | 'sampling'
  | 'elicitation'
  | 'progress'
  | 'logging'
  | 'resourceUpdates'
  | 'completions';

/**
 * Feature availability information
 */
export interface FeatureAvailability {
  /** Is the feature currently available? */
  available: boolean;
  /** Does the transport support this feature? */
  transportSupports: boolean;
  /** Did the server advertise this capability? */
  serverAdvertises: boolean;
  /** User-friendly message explaining availability */
  message: string;
  /** Detailed explanation (for tooltips) */
  detailedExplanation?: string;
}

/**
 * Server capabilities interface (matching backend)
 */
export interface ServerCapabilities {
  tools?: { list_changed?: boolean };
  resources?: { subscribe?: boolean; list_changed?: boolean };
  prompts?: { list_changed?: boolean };
  sampling?: {};
  elicitation?: {};
}

/**
 * Check if a specific feature is available based on transport and server capabilities
 *
 * @param feature - The MCP feature to check
 * @param transportType - The transport type (stdio, http, websocket, tcp, unix)
 * @param serverCapabilities - Server's advertised capabilities
 * @returns Feature availability information
 */
export function getFeatureAvailability(
  feature: MCPFeature,
  transportType: string,
  serverCapabilities: ServerCapabilities | undefined
): FeatureAvailability {
  const transportInfo = TRANSPORT_CAPABILITIES[transportType];

  if (!transportInfo) {
    return {
      available: false,
      transportSupports: false,
      serverAdvertises: false,
      message: 'Unknown transport type',
      detailedExplanation: `Transport type '${transportType}' is not recognized.`
    };
  }

  // Check if transport supports the feature
  // Note: For most features, all transports support them
  // Sampling and elicitation require bidirectional communication
  let transportSupports = true;

  if (feature === 'sampling' || feature === 'elicitation') {
    transportSupports = transportInfo.bidirectional;
  }

  // Check if server advertises the capability
  let serverAdvertises = false;

  switch (feature) {
    case 'tools':
      serverAdvertises = serverCapabilities?.tools !== undefined;
      break;
    case 'prompts':
      serverAdvertises = serverCapabilities?.prompts !== undefined;
      break;
    case 'resources':
      serverAdvertises = serverCapabilities?.resources !== undefined;
      break;
    case 'sampling':
      serverAdvertises = serverCapabilities?.sampling !== undefined;
      break;
    case 'elicitation':
      serverAdvertises = serverCapabilities?.elicitation !== undefined;
      break;
    default:
      // Other features (progress, logging, etc.) are always available if connected
      serverAdvertises = true;
  }

  const available = transportSupports && serverAdvertises;

  // Generate user-friendly message
  let message = '';
  let detailedExplanation = '';

  if (!available) {
    if (!transportSupports) {
      message = `${feature} requires bidirectional communication`;
      detailedExplanation = `The ${feature} feature requires bidirectional communication (server → client requests). ` +
        `The ${transportInfo.name} transport does not support this.`;
    } else if (!serverAdvertises) {
      message = `Server has not advertised ${feature} capability`;
      detailedExplanation = `The ${transportInfo.name} transport supports ${feature}, but this server has not ` +
        `advertised the '${feature}' capability in its initialization response. ` +
        `The server may not implement this feature.`;
    }
  } else {
    message = `${feature} is available`;
    detailedExplanation = `The ${transportInfo.name} transport supports ${feature} and the server has advertised this capability.`;
  }

  return {
    available,
    transportSupports,
    serverAdvertises,
    message,
    detailedExplanation
  };
}

/**
 * Get a user-friendly description of why a feature is unavailable
 *
 * @param feature - The MCP feature
 * @param availability - Feature availability information
 * @returns User-friendly explanation
 */
export function getUnavailabilityReason(
  feature: MCPFeature,
  availability: FeatureAvailability
): string {
  if (availability.available) {
    return '';  // Feature is available
  }

  if (!availability.transportSupports) {
    return `Transport limitation: ${feature} requires bidirectional communication`;
  }

  if (!availability.serverAdvertises) {
    return `Server limitation: Server has not advertised ${feature} capability`;
  }

  return 'Feature unavailable (unknown reason)';
}

/**
 * Get all features and their availability for a server
 *
 * @param transportType - The transport type
 * @param serverCapabilities - Server's advertised capabilities
 * @returns Map of features to their availability
 */
export function getAllFeatureAvailability(
  transportType: string,
  serverCapabilities: ServerCapabilities | undefined
): Record<MCPFeature, FeatureAvailability> {
  const features: MCPFeature[] = [
    'tools',
    'prompts',
    'resources',
    'sampling',
    'elicitation',
    'progress',
    'logging',
    'resourceUpdates',
    'completions'
  ];

  const result: Partial<Record<MCPFeature, FeatureAvailability>> = {};

  for (const feature of features) {
    result[feature] = getFeatureAvailability(feature, transportType, serverCapabilities);
  }

  return result as Record<MCPFeature, FeatureAvailability>;
}

/**
 * Get transport information
 *
 * @param transportType - The transport type (stdio, http, websocket, tcp, unix)
 * @returns Transport information or undefined if not found
 */
export function getTransportInfo(transportType: string): TransportInfo | undefined {
  return TRANSPORT_CAPABILITIES[transportType];
}

/**
 * Check if a transport supports bidirectional communication
 *
 * @param transportType - The transport type
 * @returns true if transport is bidirectional
 */
export function isTransportBidirectional(transportType: string): boolean {
  return TRANSPORT_CAPABILITIES[transportType]?.bidirectional ?? false;
}

/**
 * Get recommended transport for a use case
 *
 * @param useCase - The use case description
 * @returns Recommended transport type
 */
export function getRecommendedTransport(useCase: string): string {
  const useCaseLower = useCase.toLowerCase();

  if (useCaseLower.includes('cli') || useCaseLower.includes('command') || useCaseLower.includes('local')) {
    return 'stdio';
  }

  if (useCaseLower.includes('web') && useCaseLower.includes('real') || useCaseLower.includes('websocket')) {
    return 'websocket';
  }

  if (useCaseLower.includes('cloud') || useCaseLower.includes('http') || useCaseLower.includes('api')) {
    return 'http';
  }

  if (useCaseLower.includes('docker') || useCaseLower.includes('container') || useCaseLower.includes('ipc')) {
    return 'unix';
  }

  if (useCaseLower.includes('network') || useCaseLower.includes('service')) {
    return 'tcp';
  }

  return 'stdio';  // Default to STDIO for local use
}
