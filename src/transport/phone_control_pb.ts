import {
  LongType,
  Message,
  MethodKind,
  ScalarType,
  proto3,
  type BinaryReadOptions,
  type FieldList,
  type JsonReadOptions,
  type JsonValue,
  type PartialMessage,
  type ServiceType,
} from '@bufbuild/protobuf';

export class SystemStatusRequest extends Message<SystemStatusRequest> {
  constructor(data?: PartialMessage<SystemStatusRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.SystemStatusRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => []);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): SystemStatusRequest {
    return new SystemStatusRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): SystemStatusRequest {
    return new SystemStatusRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): SystemStatusRequest {
    return new SystemStatusRequest().fromJsonString(jsonString, options);
  }

  static equals(a: SystemStatusRequest | undefined, b: SystemStatusRequest | undefined): boolean {
    return proto3.util.equals(SystemStatusRequest, a, b);
  }
}

export class SystemStatusResponse extends Message<SystemStatusResponse> {
  totalMemoryBytes = '0';
  usedMemoryBytes = '0';
  cpuUsagePct = 0;
  brainProvider = '';
  brainModel = '';
  memoryEntryCount = 0;

  constructor(data?: PartialMessage<SystemStatusResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.SystemStatusResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'total_memory_bytes', kind: 'scalar', T: ScalarType.UINT64, L: LongType.STRING },
    { no: 2, name: 'used_memory_bytes', kind: 'scalar', T: ScalarType.UINT64, L: LongType.STRING },
    { no: 3, name: 'cpu_usage_pct', kind: 'scalar', T: ScalarType.FLOAT },
    { no: 4, name: 'brain_provider', kind: 'scalar', T: ScalarType.STRING },
    { no: 5, name: 'brain_model', kind: 'scalar', T: ScalarType.STRING },
    { no: 6, name: 'memory_entry_count', kind: 'scalar', T: ScalarType.UINT32 },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): SystemStatusResponse {
    return new SystemStatusResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): SystemStatusResponse {
    return new SystemStatusResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): SystemStatusResponse {
    return new SystemStatusResponse().fromJsonString(jsonString, options);
  }

  static equals(a: SystemStatusResponse | undefined, b: SystemStatusResponse | undefined): boolean {
    return proto3.util.equals(SystemStatusResponse, a, b);
  }
}

export class ListWorkspacesRequest extends Message<ListWorkspacesRequest> {
  constructor(data?: PartialMessage<ListWorkspacesRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ListWorkspacesRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => []);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ListWorkspacesRequest {
    return new ListWorkspacesRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ListWorkspacesRequest {
    return new ListWorkspacesRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ListWorkspacesRequest {
    return new ListWorkspacesRequest().fromJsonString(jsonString, options);
  }

  static equals(a: ListWorkspacesRequest | undefined, b: ListWorkspacesRequest | undefined): boolean {
    return proto3.util.equals(ListWorkspacesRequest, a, b);
  }
}

export class VsCodeWorkspace extends Message<VsCodeWorkspace> {
  path = '';
  name = '';
  lastOpenedUnixMs = '0';

  constructor(data?: PartialMessage<VsCodeWorkspace>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.VsCodeWorkspace';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'path', kind: 'scalar', T: ScalarType.STRING },
    { no: 2, name: 'name', kind: 'scalar', T: ScalarType.STRING },
    { no: 3, name: 'last_opened_unix_ms', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): VsCodeWorkspace {
    return new VsCodeWorkspace().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): VsCodeWorkspace {
    return new VsCodeWorkspace().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): VsCodeWorkspace {
    return new VsCodeWorkspace().fromJsonString(jsonString, options);
  }

  static equals(a: VsCodeWorkspace | undefined, b: VsCodeWorkspace | undefined): boolean {
    return proto3.util.equals(VsCodeWorkspace, a, b);
  }
}

export class ListWorkspacesResponse extends Message<ListWorkspacesResponse> {
  workspaces: VsCodeWorkspace[] = [];

  constructor(data?: PartialMessage<ListWorkspacesResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ListWorkspacesResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'workspaces', kind: 'message', T: VsCodeWorkspace, repeated: true },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ListWorkspacesResponse {
    return new ListWorkspacesResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ListWorkspacesResponse {
    return new ListWorkspacesResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ListWorkspacesResponse {
    return new ListWorkspacesResponse().fromJsonString(jsonString, options);
  }

  static equals(a: ListWorkspacesResponse | undefined, b: ListWorkspacesResponse | undefined): boolean {
    return proto3.util.equals(ListWorkspacesResponse, a, b);
  }
}

export class CopilotSessionRequest extends Message<CopilotSessionRequest> {
  workspacePath = '';

  constructor(data?: PartialMessage<CopilotSessionRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.CopilotSessionRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'workspace_path', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): CopilotSessionRequest {
    return new CopilotSessionRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): CopilotSessionRequest {
    return new CopilotSessionRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): CopilotSessionRequest {
    return new CopilotSessionRequest().fromJsonString(jsonString, options);
  }

  static equals(a: CopilotSessionRequest | undefined, b: CopilotSessionRequest | undefined): boolean {
    return proto3.util.equals(CopilotSessionRequest, a, b);
  }
}

export class CopilotSessionResponse extends Message<CopilotSessionResponse> {
  found = false;
  workspaceFolder = '';
  sessionId = '';
  model = '';
  lastUserTurnTs = '';
  lastUserPreview = '';
  lastAssistantTurnTs = '';
  lastAssistantPreview = '';
  toolInvocationCount = 0;
  eventCount = 0;

  constructor(data?: PartialMessage<CopilotSessionResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.CopilotSessionResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'found', kind: 'scalar', T: ScalarType.BOOL },
    { no: 2, name: 'workspace_folder', kind: 'scalar', T: ScalarType.STRING },
    { no: 3, name: 'session_id', kind: 'scalar', T: ScalarType.STRING },
    { no: 4, name: 'model', kind: 'scalar', T: ScalarType.STRING },
    { no: 5, name: 'last_user_turn_ts', kind: 'scalar', T: ScalarType.STRING },
    { no: 6, name: 'last_user_preview', kind: 'scalar', T: ScalarType.STRING },
    { no: 7, name: 'last_assistant_turn_ts', kind: 'scalar', T: ScalarType.STRING },
    { no: 8, name: 'last_assistant_preview', kind: 'scalar', T: ScalarType.STRING },
    { no: 9, name: 'tool_invocation_count', kind: 'scalar', T: ScalarType.UINT32 },
    { no: 10, name: 'event_count', kind: 'scalar', T: ScalarType.UINT32 },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): CopilotSessionResponse {
    return new CopilotSessionResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): CopilotSessionResponse {
    return new CopilotSessionResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): CopilotSessionResponse {
    return new CopilotSessionResponse().fromJsonString(jsonString, options);
  }

  static equals(a: CopilotSessionResponse | undefined, b: CopilotSessionResponse | undefined): boolean {
    return proto3.util.equals(CopilotSessionResponse, a, b);
  }
}

export class ListWorkflowsRequest extends Message<ListWorkflowsRequest> {
  includeFinished = false;

  constructor(data?: PartialMessage<ListWorkflowsRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ListWorkflowsRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'include_finished', kind: 'scalar', T: ScalarType.BOOL },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ListWorkflowsRequest {
    return new ListWorkflowsRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ListWorkflowsRequest {
    return new ListWorkflowsRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ListWorkflowsRequest {
    return new ListWorkflowsRequest().fromJsonString(jsonString, options);
  }

  static equals(a: ListWorkflowsRequest | undefined, b: ListWorkflowsRequest | undefined): boolean {
    return proto3.util.equals(ListWorkflowsRequest, a, b);
  }
}

export class WorkflowRun extends Message<WorkflowRun> {
  workflowId = '';
  name = '';
  status = '';
  startedAtUnixMs = '0';
  lastEventAtUnixMs = '0';
  eventCount = '0';

  constructor(data?: PartialMessage<WorkflowRun>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.WorkflowRun';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'workflow_id', kind: 'scalar', T: ScalarType.STRING },
    { no: 2, name: 'name', kind: 'scalar', T: ScalarType.STRING },
    { no: 3, name: 'status', kind: 'scalar', T: ScalarType.STRING },
    { no: 4, name: 'started_at_unix_ms', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 5, name: 'last_event_at_unix_ms', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 6, name: 'event_count', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): WorkflowRun {
    return new WorkflowRun().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): WorkflowRun {
    return new WorkflowRun().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): WorkflowRun {
    return new WorkflowRun().fromJsonString(jsonString, options);
  }

  static equals(a: WorkflowRun | undefined, b: WorkflowRun | undefined): boolean {
    return proto3.util.equals(WorkflowRun, a, b);
  }
}

export class ListWorkflowsResponse extends Message<ListWorkflowsResponse> {
  runs: WorkflowRun[] = [];

  constructor(data?: PartialMessage<ListWorkflowsResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ListWorkflowsResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'runs', kind: 'message', T: WorkflowRun, repeated: true },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ListWorkflowsResponse {
    return new ListWorkflowsResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ListWorkflowsResponse {
    return new ListWorkflowsResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ListWorkflowsResponse {
    return new ListWorkflowsResponse().fromJsonString(jsonString, options);
  }

  static equals(a: ListWorkflowsResponse | undefined, b: ListWorkflowsResponse | undefined): boolean {
    return proto3.util.equals(ListWorkflowsResponse, a, b);
  }
}

export class WorkflowProgressRequest extends Message<WorkflowProgressRequest> {
  workflowId = '';

  constructor(data?: PartialMessage<WorkflowProgressRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.WorkflowProgressRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'workflow_id', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): WorkflowProgressRequest {
    return new WorkflowProgressRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): WorkflowProgressRequest {
    return new WorkflowProgressRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): WorkflowProgressRequest {
    return new WorkflowProgressRequest().fromJsonString(jsonString, options);
  }

  static equals(a: WorkflowProgressRequest | undefined, b: WorkflowProgressRequest | undefined): boolean {
    return proto3.util.equals(WorkflowProgressRequest, a, b);
  }
}

export class WorkflowProgressResponse extends Message<WorkflowProgressResponse> {
  workflowId = '';
  name = '';
  status = '';
  startedAtUnixMs = '0';
  lastEventAtUnixMs = '0';
  eventCount = '0';
  summary = '';

  constructor(data?: PartialMessage<WorkflowProgressResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.WorkflowProgressResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'workflow_id', kind: 'scalar', T: ScalarType.STRING },
    { no: 2, name: 'name', kind: 'scalar', T: ScalarType.STRING },
    { no: 3, name: 'status', kind: 'scalar', T: ScalarType.STRING },
    { no: 4, name: 'started_at_unix_ms', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 5, name: 'last_event_at_unix_ms', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 6, name: 'event_count', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 7, name: 'summary', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): WorkflowProgressResponse {
    return new WorkflowProgressResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): WorkflowProgressResponse {
    return new WorkflowProgressResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): WorkflowProgressResponse {
    return new WorkflowProgressResponse().fromJsonString(jsonString, options);
  }

  static equals(a: WorkflowProgressResponse | undefined, b: WorkflowProgressResponse | undefined): boolean {
    return proto3.util.equals(WorkflowProgressResponse, a, b);
  }
}

export class ContinueWorkflowRequest extends Message<ContinueWorkflowRequest> {
  workflowId = '';

  constructor(data?: PartialMessage<ContinueWorkflowRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ContinueWorkflowRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'workflow_id', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ContinueWorkflowRequest {
    return new ContinueWorkflowRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ContinueWorkflowRequest {
    return new ContinueWorkflowRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ContinueWorkflowRequest {
    return new ContinueWorkflowRequest().fromJsonString(jsonString, options);
  }

  static equals(a: ContinueWorkflowRequest | undefined, b: ContinueWorkflowRequest | undefined): boolean {
    return proto3.util.equals(ContinueWorkflowRequest, a, b);
  }
}

export class ContinueWorkflowResponse extends Message<ContinueWorkflowResponse> {
  accepted = false;
  message = '';

  constructor(data?: PartialMessage<ContinueWorkflowResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ContinueWorkflowResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'accepted', kind: 'scalar', T: ScalarType.BOOL },
    { no: 2, name: 'message', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ContinueWorkflowResponse {
    return new ContinueWorkflowResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ContinueWorkflowResponse {
    return new ContinueWorkflowResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ContinueWorkflowResponse {
    return new ContinueWorkflowResponse().fromJsonString(jsonString, options);
  }

  static equals(a: ContinueWorkflowResponse | undefined, b: ContinueWorkflowResponse | undefined): boolean {
    return proto3.util.equals(ContinueWorkflowResponse, a, b);
  }
}

export class ChatRequest extends Message<ChatRequest> {
  message = '';

  constructor(data?: PartialMessage<ChatRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ChatRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'message', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ChatRequest {
    return new ChatRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ChatRequest {
    return new ChatRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ChatRequest {
    return new ChatRequest().fromJsonString(jsonString, options);
  }

  static equals(a: ChatRequest | undefined, b: ChatRequest | undefined): boolean {
    return proto3.util.equals(ChatRequest, a, b);
  }
}

export class ChatResponse extends Message<ChatResponse> {
  reply = '';

  constructor(data?: PartialMessage<ChatResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ChatResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'reply', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ChatResponse {
    return new ChatResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ChatResponse {
    return new ChatResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ChatResponse {
    return new ChatResponse().fromJsonString(jsonString, options);
  }

  static equals(a: ChatResponse | undefined, b: ChatResponse | undefined): boolean {
    return proto3.util.equals(ChatResponse, a, b);
  }
}

export class ChatChunk extends Message<ChatChunk> {
  text = '';
  done = false;

  constructor(data?: PartialMessage<ChatChunk>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ChatChunk';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'text', kind: 'scalar', T: ScalarType.STRING },
    { no: 2, name: 'done', kind: 'scalar', T: ScalarType.BOOL },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ChatChunk {
    return new ChatChunk().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ChatChunk {
    return new ChatChunk().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ChatChunk {
    return new ChatChunk().fromJsonString(jsonString, options);
  }

  static equals(a: ChatChunk | undefined, b: ChatChunk | undefined): boolean {
    return proto3.util.equals(ChatChunk, a, b);
  }
}

export class ListDevicesRequest extends Message<ListDevicesRequest> {
  constructor(data?: PartialMessage<ListDevicesRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ListDevicesRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => []);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ListDevicesRequest {
    return new ListDevicesRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ListDevicesRequest {
    return new ListDevicesRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ListDevicesRequest {
    return new ListDevicesRequest().fromJsonString(jsonString, options);
  }

  static equals(a: ListDevicesRequest | undefined, b: ListDevicesRequest | undefined): boolean {
    return proto3.util.equals(ListDevicesRequest, a, b);
  }
}

export class PairedDeviceInfo extends Message<PairedDeviceInfo> {
  deviceId = '';
  displayName = '';
  pairedAtUnixMs = '0';
  lastSeenAtUnixMs = '0';
  capabilities: string[] = [];

  constructor(data?: PartialMessage<PairedDeviceInfo>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.PairedDeviceInfo';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'device_id', kind: 'scalar', T: ScalarType.STRING },
    { no: 2, name: 'display_name', kind: 'scalar', T: ScalarType.STRING },
    { no: 3, name: 'paired_at_unix_ms', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 4, name: 'last_seen_at_unix_ms', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 5, name: 'capabilities', kind: 'scalar', T: ScalarType.STRING, repeated: true },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): PairedDeviceInfo {
    return new PairedDeviceInfo().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): PairedDeviceInfo {
    return new PairedDeviceInfo().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): PairedDeviceInfo {
    return new PairedDeviceInfo().fromJsonString(jsonString, options);
  }

  static equals(a: PairedDeviceInfo | undefined, b: PairedDeviceInfo | undefined): boolean {
    return proto3.util.equals(PairedDeviceInfo, a, b);
  }
}

export class ListDevicesResponse extends Message<ListDevicesResponse> {
  devices: PairedDeviceInfo[] = [];

  constructor(data?: PartialMessage<ListDevicesResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.phone_control.v1.ListDevicesResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'devices', kind: 'message', T: PairedDeviceInfo, repeated: true },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): ListDevicesResponse {
    return new ListDevicesResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): ListDevicesResponse {
    return new ListDevicesResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): ListDevicesResponse {
    return new ListDevicesResponse().fromJsonString(jsonString, options);
  }

  static equals(a: ListDevicesResponse | undefined, b: ListDevicesResponse | undefined): boolean {
    return proto3.util.equals(ListDevicesResponse, a, b);
  }
}

export const PhoneControlService = {
  typeName: 'terransoul.phone_control.v1.PhoneControl',
  methods: {
    getSystemStatus: {
      name: 'GetSystemStatus',
      I: SystemStatusRequest,
      O: SystemStatusResponse,
      kind: MethodKind.Unary,
    },
    listVsCodeWorkspaces: {
      name: 'ListVsCodeWorkspaces',
      I: ListWorkspacesRequest,
      O: ListWorkspacesResponse,
      kind: MethodKind.Unary,
    },
    getCopilotSessionStatus: {
      name: 'GetCopilotSessionStatus',
      I: CopilotSessionRequest,
      O: CopilotSessionResponse,
      kind: MethodKind.Unary,
    },
    listWorkflowRuns: {
      name: 'ListWorkflowRuns',
      I: ListWorkflowsRequest,
      O: ListWorkflowsResponse,
      kind: MethodKind.Unary,
    },
    getWorkflowProgress: {
      name: 'GetWorkflowProgress',
      I: WorkflowProgressRequest,
      O: WorkflowProgressResponse,
      kind: MethodKind.Unary,
    },
    continueWorkflow: {
      name: 'ContinueWorkflow',
      I: ContinueWorkflowRequest,
      O: ContinueWorkflowResponse,
      kind: MethodKind.Unary,
    },
    sendChatMessage: {
      name: 'SendChatMessage',
      I: ChatRequest,
      O: ChatResponse,
      kind: MethodKind.Unary,
    },
    streamChatMessage: {
      name: 'StreamChatMessage',
      I: ChatRequest,
      O: ChatChunk,
      kind: MethodKind.ServerStreaming,
    },
    listPairedDevices: {
      name: 'ListPairedDevices',
      I: ListDevicesRequest,
      O: ListDevicesResponse,
      kind: MethodKind.Unary,
    },
  },
} as const satisfies ServiceType;