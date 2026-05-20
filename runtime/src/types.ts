export type RuntimeManifest = {
  format_version: number;
  package_kind: string;
  source_name: string;
  source_hash: string;
  engine_family: string;
  compatibility: 'supported' | 'partial' | 'blocked';
  default_room_id: number | null;
  room_count: number;
  object_count: number;
  script_block_count: number;
  sprite_count: number;
  background_count: number;
  sound_count: number;
  resource_index_path: string;
  warnings: string[];
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
  }>;
};

export type RoomDefinition = {
  id: number;
  name: string;
  width: number;
  height: number;
  speed: number;
  persistent: boolean;
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
  resources: ResourceIndex;
  analysis: RuntimeAnalysis;
};
