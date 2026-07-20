export type CompatibilityLevel = 'supported' | 'partial' | 'blocked';

export type RuntimeManifest = {
  format_version: number;
  package_kind: string;
  source_name: string;
  source_hash: string;
  engine_family: string;
  compatibility: CompatibilityLevel;
  default_room_id: number | null;
  room_order?: number[];
  room_count: number;
  object_count: number;
  script_block_count: number;
  sprite_count: number;
  background_count: number;
  sound_count: number;
  resource_index_path: string;
  warnings: string[];
  /** Source of the manifest display size. */
  display_source?: 'exe-resolution' | 'default-room';
  /** Manifest display width, from EXE resolution or default-room fallback. */
  display_width?: number;
  /** Manifest display height, from EXE resolution or default-room fallback. */
  display_height?: number;
  zero_uninitialized_vars?: boolean;
};

export type ResourceIndex = {
  sprites: Array<{
    id: number;
    name: string;
    origin_x: number;
    origin_y: number;
    frame_paths: string[];
    width: number;
    height: number;
    bbox_left: number;
    bbox_right: number;
    bbox_top: number;
    bbox_bottom: number;
    collision_masks?: Array<{
      width: number;
      height: number;
      bbox_left: number;
      bbox_right: number;
      bbox_top: number;
      bbox_bottom: number;
      data: boolean[];
    }>;
    per_frame_collision_masks?: boolean;
  }>;
  backgrounds: Array<{
    id: number;
    name: string;
    width: number;
    height: number;
    image_path: string;
  }>;
  sounds: Array<{
    id: number;
    name: string;
    file_path: string;
    extension: string;
    preload: boolean;
    kind?: 'normal' | 'background-music' | 'three-dimensional' | 'multimedia';
  }>;
  fonts?: Array<{
    id: number;
    name: string;
    system_name: string;
    size: number;
    bold: boolean;
    italic: boolean;
    range_start?: number;
    range_end?: number;
    map_width?: number;
    map_height?: number;
    image_path?: string;
    glyphs?: Array<{
      code: number;
      x: number;
      y: number;
      width: number;
      height: number;
      offset: number;
      advance: number;
    }>;
  }>;
  paths?: Array<{
    id: number;
    name: string;
    smooth: boolean;
    precision: number;
    closed: boolean;
    points: Array<{ x: number; y: number; speed: number }>;
  }>;
};

export type RoomDefinition = {
  id: number;
  name: string;
  width: number;
  height: number;
  speed: number;
  persistent: boolean;
  background_colour?: number;
  clear_screen?: boolean;
  backgrounds: Array<{
    visible_on_start: boolean;
    is_foreground: boolean;
    source_bg: number;
    xoffset: number;
    yoffset: number;
    tile_horz: boolean;
    tile_vert: boolean;
    hspeed: number;
    vspeed: number;
    stretch: boolean;
  }>;
  views_enabled: boolean;
  views: Array<{
    visible: boolean;
    source_x: number;
    source_y: number;
    source_w: number;
    source_h: number;
    port_x: number;
    port_y: number;
    port_w: number;
    port_h: number;
    target: number;
    hborder?: number;
    vborder?: number;
    hspeed?: number;
    vspeed?: number;
  }>;
  tiles: Array<{
    tile_id: number;
    source_bg: number;
    x: number;
    y: number;
    tile_x: number;
    tile_y: number;
    width: number;
    height: number;
    depth: number;
    xscale: number;
    yscale: number;
    blend: number;
  }>;
  instances: Array<{
    instance_id: number;
    object_id: number;
    x: number;
    y: number;
    xscale: number;
    yscale: number;
    angle: number;
    blend: number;
    creation_block_id: string | null;
    /** Runtime hint: whether this instance is solid */
    is_solid: boolean;
    /** Runtime hint: whether this instance is a hazard */
    is_hazard: boolean;
    /** Runtime hint: whether this instance is a checkpoint */
    is_checkpoint: boolean;
  }>;
  creation_block_id: string | null;
  /** Runtime hint: whether this room is playable */
  playable: boolean;
  /** Runtime hint: room IDs this room can transition to */
  transition_targets: number[];
};

export type ObjectDefinition = {
  id: number;
  name: string;
  sprite_index: number;
  parent_index: number;
  depth: number;
  persistent: boolean;
  visible: boolean;
  solid: boolean;
  mask_index: number;
  /** Runtime hint: whether this object is a hazard (null if cannot determine) */
  is_hazard: boolean | null;
  /** Runtime hint: whether this object is a checkpoint (null if cannot determine) */
  is_checkpoint: boolean | null;
  /** Runtime hint: whether this object is player-controlled */
  is_player: boolean;
  events: Array<{
    event_type: number;
    sub_event: number;
    /** Normalized event tag for runtime dispatch */
    event_tag: string;
    block_id: string;
    action_count: number;
  }>;
};

export type ScriptIrFile = {
  format: string;
  blocks: Array<{
    id: string;
    name: string;
    kind: string;
    /** Support level: "action-list" (executable) or "source-only" (requires GML lowering) */
    support: string;
    /** Count of actions that can be executed without GML lowering */
    executable_action_count: number;
    ops: Array<Record<string, unknown>>;
  }>;
};

export type RuntimeRawCodeAction = {
  action_id: number;
  lib_id: number;
  action_kind: number;
  execution_type: number;
  applies_to?: number;
  is_condition?: boolean;
  invert_condition?: boolean;
  is_relative?: boolean;
  fn_name: string;
  fn_code: string;
  args: string[];
};

export type RuntimeRawLogicFile = {
  format: string;
  room_creation_codes: Array<{
    owner_kind: string;
    owner_id: number;
    owner_name: string;
    event_type: number | null;
    sub_event: number | null;
    collision_object_id: number | null;
    block_id: string;
    gml_source: string;
  }>;
  instance_creation_codes: Array<{
    owner_kind: string;
    owner_id: number;
    owner_name: string;
    event_type: number | null;
    sub_event: number | null;
    collision_object_id: number | null;
    block_id: string;
    gml_source: string;
  }>;
  object_events: Array<{
    object_id: number;
    object_name: string;
    event_type: number;
    sub_event: number;
    event_tag: string;
    collision_object_id: number | null;
    block_id: string;
    actions: RuntimeRawCodeAction[];
  }>;
  scripts: Array<{
    script_id: number;
    script_name: string;
    gml_source: string;
  }>;
  triggers: Array<{
    trigger_id: number;
    trigger_name: string;
    constant_name: string;
    moment: string;
    condition_gml: string;
  }>;
  timelines: Array<{
    timeline_id: number;
    timeline_name: string;
    moment: number;
    actions: RuntimeRawCodeAction[];
  }>;
};

export type RuntimeLoweredLogicFile = {
  format: string;
  entries: Array<{
    block_id: string;
    statements: RuntimeLoweredLogicStatement[];
  }>;
};

export type RuntimeLoweredLogicExpr =
  {
    kind: 'identifier';
    value: string;
  }
  | {
    kind: 'literal-number';
    value: number;
  }
  | {
    kind: 'literal-bool';
    value: boolean;
  }
  | {
    kind: 'literal-text';
    value: string;
  }
  | {
    kind: 'call';
    value: {
      name: string;
      args: RuntimeLoweredLogicExpr[];
    };
  }
  | {
    kind: 'member-access';
    value: {
      target: RuntimeLoweredLogicExpr;
      member: string;
    };
  }
  | {
    kind: 'index-access';
    value: {
      target: RuntimeLoweredLogicExpr;
      index: RuntimeLoweredLogicExpr;
    };
  }
  | {
    kind: 'binary-expr';
    value: {
      op: string;
      left: RuntimeLoweredLogicExpr;
      right: RuntimeLoweredLogicExpr;
    };
  }
  | {
    kind: 'raw';
    value: {
      source: string;
    };
  };

export type RuntimeLoweredLogicStatement =
  | {
    kind: 'assignment';
    target: RuntimeLoweredLogicExpr;
    value: RuntimeLoweredLogicExpr;
  }
  | {
    kind: 'function-call';
    name: string;
    args: RuntimeLoweredLogicExpr[];
  }
  | {
    kind: 'conditional';
    condition: RuntimeLoweredLogicExpr;
    then_branch: RuntimeLoweredLogicStatement[];
    else_branch: RuntimeLoweredLogicStatement[];
  }
  | {
    kind: 'with';
    target: RuntimeLoweredLogicExpr;
    body: RuntimeLoweredLogicStatement[];
  }
  | {
    kind: 'repeat';
    count: RuntimeLoweredLogicExpr;
    body: RuntimeLoweredLogicStatement[];
  }
  | {
    kind: 'while';
    condition: RuntimeLoweredLogicExpr;
    body: RuntimeLoweredLogicStatement[];
  }
  | {
    kind: 'for';
    init: RuntimeLoweredLogicExpr;
    condition: RuntimeLoweredLogicExpr;
    step: RuntimeLoweredLogicExpr;
    body: RuntimeLoweredLogicStatement[];
  }
  | {
    kind: 'raw';
    source: string;
  };

export type RuntimeAnalysis = {
  dlls: string[];
  included_files: string[];
  warnings: string[];
  unsupported_features: string[];
};

export type RuntimePackage = {
  manifest: RuntimeManifest;
  rooms: RoomDefinition[];
  objects: ObjectDefinition[];
  scripts: ScriptIrFile;
  rawLogic: RuntimeRawLogicFile;
  loweredLogic: RuntimeLoweredLogicFile;
  resources: ResourceIndex;
  analysis: RuntimeAnalysis;
};
