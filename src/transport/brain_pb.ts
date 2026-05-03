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

export enum SearchMode {
  Rrf = 0,
  Hybrid = 1,
  Hyde = 2,
}

proto3.util.setEnumType(SearchMode, 'terransoul.brain.v1.SearchMode', [
  { no: 0, name: 'SEARCH_MODE_RRF' },
  { no: 1, name: 'SEARCH_MODE_HYBRID' },
  { no: 2, name: 'SEARCH_MODE_HYDE' },
]);

export class HealthRequest extends Message<HealthRequest> {
  constructor(data?: PartialMessage<HealthRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.brain.v1.HealthRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => []);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): HealthRequest {
    return new HealthRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): HealthRequest {
    return new HealthRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): HealthRequest {
    return new HealthRequest().fromJsonString(jsonString, options);
  }

  static equals(a: HealthRequest | undefined, b: HealthRequest | undefined): boolean {
    return proto3.util.equals(HealthRequest, a, b);
  }
}

export class HealthResponse extends Message<HealthResponse> {
  version = '';
  brainProvider = '';
  brainModel?: string;
  ragQualityPct = 0;
  memoryTotal = '0';

  constructor(data?: PartialMessage<HealthResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.brain.v1.HealthResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'version', kind: 'scalar', T: ScalarType.STRING },
    { no: 2, name: 'brain_provider', kind: 'scalar', T: ScalarType.STRING },
    { no: 3, name: 'brain_model', kind: 'scalar', T: ScalarType.STRING, opt: true },
    { no: 4, name: 'rag_quality_pct', kind: 'scalar', T: ScalarType.UINT32 },
    { no: 5, name: 'memory_total', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): HealthResponse {
    return new HealthResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): HealthResponse {
    return new HealthResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): HealthResponse {
    return new HealthResponse().fromJsonString(jsonString, options);
  }

  static equals(a: HealthResponse | undefined, b: HealthResponse | undefined): boolean {
    return proto3.util.equals(HealthResponse, a, b);
  }
}

export class SearchRequest extends Message<SearchRequest> {
  query = '';
  limit?: number;
  mode = SearchMode.Rrf;

  constructor(data?: PartialMessage<SearchRequest>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.brain.v1.SearchRequest';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'query', kind: 'scalar', T: ScalarType.STRING },
    { no: 2, name: 'limit', kind: 'scalar', T: ScalarType.UINT32, opt: true },
    { no: 3, name: 'mode', kind: 'enum', T: proto3.getEnumType(SearchMode) },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): SearchRequest {
    return new SearchRequest().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): SearchRequest {
    return new SearchRequest().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): SearchRequest {
    return new SearchRequest().fromJsonString(jsonString, options);
  }

  static equals(a: SearchRequest | undefined, b: SearchRequest | undefined): boolean {
    return proto3.util.equals(SearchRequest, a, b);
  }
}

export class SearchHit extends Message<SearchHit> {
  id = '0';
  content = '';
  tags = '';
  importance = '0';
  score = 0;
  sourceUrl?: string;
  tier = '';

  constructor(data?: PartialMessage<SearchHit>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.brain.v1.SearchHit';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'id', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 2, name: 'content', kind: 'scalar', T: ScalarType.STRING },
    { no: 3, name: 'tags', kind: 'scalar', T: ScalarType.STRING },
    { no: 4, name: 'importance', kind: 'scalar', T: ScalarType.INT64, L: LongType.STRING },
    { no: 5, name: 'score', kind: 'scalar', T: ScalarType.DOUBLE },
    { no: 6, name: 'source_url', kind: 'scalar', T: ScalarType.STRING, opt: true },
    { no: 7, name: 'tier', kind: 'scalar', T: ScalarType.STRING },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): SearchHit {
    return new SearchHit().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): SearchHit {
    return new SearchHit().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): SearchHit {
    return new SearchHit().fromJsonString(jsonString, options);
  }

  static equals(a: SearchHit | undefined, b: SearchHit | undefined): boolean {
    return proto3.util.equals(SearchHit, a, b);
  }
}

export class SearchResponse extends Message<SearchResponse> {
  hits: SearchHit[] = [];

  constructor(data?: PartialMessage<SearchResponse>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime = proto3;
  static readonly typeName = 'terransoul.brain.v1.SearchResponse';
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: 'hits', kind: 'message', T: SearchHit, repeated: true },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): SearchResponse {
    return new SearchResponse().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): SearchResponse {
    return new SearchResponse().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): SearchResponse {
    return new SearchResponse().fromJsonString(jsonString, options);
  }

  static equals(a: SearchResponse | undefined, b: SearchResponse | undefined): boolean {
    return proto3.util.equals(SearchResponse, a, b);
  }
}

export const BrainService = {
  typeName: 'terransoul.brain.v1.Brain',
  methods: {
    health: {
      name: 'Health',
      I: HealthRequest,
      O: HealthResponse,
      kind: MethodKind.Unary,
    },
    search: {
      name: 'Search',
      I: SearchRequest,
      O: SearchResponse,
      kind: MethodKind.Unary,
    },
    streamSearch: {
      name: 'StreamSearch',
      I: SearchRequest,
      O: SearchHit,
      kind: MethodKind.ServerStreaming,
    },
  },
} as const satisfies ServiceType;