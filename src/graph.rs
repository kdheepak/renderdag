use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub parents: Vec<NodeId>,
}

impl Node {
    #[inline]
    pub fn new(id: impl Into<NodeId>, parents: Vec<NodeId>) -> Self {
        Self {
            id: id.into(),
            parents,
        }
    }
}

impl GraphNode for Node {
    type Id = NodeId;

    #[inline]
    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    #[inline]
    fn parents(&self) -> &[Self::Id] {
        &self.parents
    }
}

pub type LaneId = u32;
pub type NodeId = String;

pub trait GraphNode {
    type Id: Eq + Hash + Clone;

    fn id(&self) -> Self::Id;
    fn parents(&self) -> &[Self::Id];
}

fn normalize_key(raw_key: &str) -> String {
    let mut normalized_key = String::with_capacity(raw_key.len());
    for character in raw_key.chars() {
        if character == '_' || character == '-' || character == ' ' {
            continue;
        }
        normalized_key.push(character.to_ascii_lowercase());
    }
    normalized_key
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    Initial = 0,
    Merge = 1,
    MergeLeaf = 2,
    Node = 3,
    NodeLeaf = 4,
    Orphan = 5,
    MergeTruncated = 6,
    MergeLeafTruncated = 7,
}

impl NodeKind {
    pub const COUNT: usize = 8;

    #[inline]
    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn as_snake(self) -> &'static str {
        match self {
            NodeKind::Initial => "initial",
            NodeKind::Merge => "merge",
            NodeKind::MergeLeaf => "merge_leaf",
            NodeKind::Node => "node",
            NodeKind::NodeLeaf => "node_leaf",
            NodeKind::Orphan => "orphan",
            NodeKind::MergeTruncated => "merge_truncated",
            NodeKind::MergeLeafTruncated => "merge_leaf_truncated",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match normalize_key(input).as_str() {
            "initial" => Some(NodeKind::Initial),
            "merge" => Some(NodeKind::Merge),
            "mergeleaf" => Some(NodeKind::MergeLeaf),
            "node" => Some(NodeKind::Node),
            "nodeleaf" => Some(NodeKind::NodeLeaf),
            "orphan" => Some(NodeKind::Orphan),
            "mergetruncated" => Some(NodeKind::MergeTruncated),
            "mergeleaftruncated" => Some(NodeKind::MergeLeafTruncated),
            _ => None,
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for NodeKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_snake())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for NodeKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value = String::deserialize(deserializer)?;
        NodeKind::parse(&raw_value).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid NodeKind '{raw_value}'. expected one of: initial, merge, merge_leaf, node, node_leaf, orphan, merge_truncated, merge_leaf_truncated"
            ))
        })
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MissingParentState {
    #[default]
    None = 0,
    Some = 1,
    All = 2,
}

impl MissingParentState {
    pub const fn as_snake(self) -> &'static str {
        match self {
            MissingParentState::None => "none",
            MissingParentState::Some => "some",
            MissingParentState::All => "all",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match normalize_key(input).as_str() {
            "none" => Some(MissingParentState::None),
            "some" => Some(MissingParentState::Some),
            "all" => Some(MissingParentState::All),
            _ => None,
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for MissingParentState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_snake())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MissingParentState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value = String::deserialize(deserializer)?;
        MissingParentState::parse(&raw_value).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid MissingParentState '{raw_value}'. expected one of: none, some, all"
            ))
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ParentAvailability {
    pub present_count: usize,
    pub missing_count: usize,
}

impl ParentAvailability {
    #[inline]
    pub const fn new(present_count: usize, missing_count: usize) -> Self {
        Self {
            present_count,
            missing_count,
        }
    }

    #[inline]
    pub const fn missing_parent_state(self) -> MissingParentState {
        if self.missing_count == 0 {
            MissingParentState::None
        } else if self.present_count == 0 {
            MissingParentState::All
        } else {
            MissingParentState::Some
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConnectionKind {
    Empty = 0,
    Horizontal = 1,
    Vertical = 2,
    CornerUpLeft = 3,
    CornerUpRight = 4,
    CornerDownRight = 5,
    CornerDownLeft = 6,
    TeeUp = 7,
    TeeDown = 8,
    TeeLeft = 9,
    TeeRight = 10,
    EndLeft = 11,
    EndRight = 12,
    EndUp = 13,
    EndDown = 14,
    CrossOver = 15,
}

impl ConnectionKind {
    pub const COUNT: usize = 16;

    #[inline]
    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn as_snake(self) -> &'static str {
        match self {
            ConnectionKind::Empty => "empty",
            ConnectionKind::Horizontal => "horizontal",
            ConnectionKind::Vertical => "vertical",
            ConnectionKind::CornerUpLeft => "corner_up_left",
            ConnectionKind::CornerUpRight => "corner_up_right",
            ConnectionKind::CornerDownRight => "corner_down_right",
            ConnectionKind::CornerDownLeft => "corner_down_left",
            ConnectionKind::TeeUp => "tee_up",
            ConnectionKind::TeeDown => "tee_down",
            ConnectionKind::TeeLeft => "tee_left",
            ConnectionKind::TeeRight => "tee_right",
            ConnectionKind::EndLeft => "end_left",
            ConnectionKind::EndRight => "end_right",
            ConnectionKind::EndUp => "end_up",
            ConnectionKind::EndDown => "end_down",
            ConnectionKind::CrossOver => "cross_over",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match normalize_key(input).as_str() {
            "empty" => Some(ConnectionKind::Empty),
            "horizontal" => Some(ConnectionKind::Horizontal),
            "vertical" => Some(ConnectionKind::Vertical),
            "cornerupleft" => Some(ConnectionKind::CornerUpLeft),
            "cornerupright" => Some(ConnectionKind::CornerUpRight),
            "cornerdownright" => Some(ConnectionKind::CornerDownRight),
            "cornerdownleft" => Some(ConnectionKind::CornerDownLeft),
            "teeup" => Some(ConnectionKind::TeeUp),
            "teedown" => Some(ConnectionKind::TeeDown),
            "teeleft" => Some(ConnectionKind::TeeLeft),
            "teeright" => Some(ConnectionKind::TeeRight),
            "endleft" => Some(ConnectionKind::EndLeft),
            "endright" => Some(ConnectionKind::EndRight),
            "endup" => Some(ConnectionKind::EndUp),
            "enddown" => Some(ConnectionKind::EndDown),
            "crossover" => Some(ConnectionKind::CrossOver),
            _ => None,
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for ConnectionKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_snake())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ConnectionKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value = String::deserialize(deserializer)?;
        ConnectionKind::parse(&raw_value).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid ConnectionKind '{raw_value}'. expected one of: empty, horizontal, vertical, corner_up_left, corner_up_right, corner_down_right, corner_down_left, tee_up, tee_down, tee_left, tee_right, end_left, end_right, end_up, end_down, cross_over"
            ))
        })
    }
}

#[derive(Clone, Debug)]
pub struct Glyphs {
    pub node: [char; NodeKind::COUNT],
    pub connection: [char; ConnectionKind::COUNT],
}

impl Default for Glyphs {
    fn default() -> Self {
        Self {
            node: [
                '⊝', // Initial
                '⊗', // Merge
                '⍟', // MergeLeaf
                '●', // Node
                '⦿', // NodeLeaf
                '◌', // Orphan
                '⊘', // MergeTruncated
                '⊛', // MergeLeafTruncated
            ],
            connection: [
                ' ', // Empty
                '─', // Horizontal
                '│', // Vertical
                '╯', // CornerUpLeft
                '╰', // CornerUpRight
                '╭', // CornerDownRight
                '╮', // CornerDownLeft
                '┴', // TeeUp
                '┬', // TeeDown
                '┤', // TeeLeft
                '├', // TeeRight
                '╴', // EndLeft
                '╶', // EndRight
                '╵', // EndUp
                '╷', // EndDown
                '┊', // CrossOver
            ],
        }
    }
}

impl Glyphs {
    #[inline]
    pub fn set_node_glyph(&mut self, kind: NodeKind, glyph: char) {
        self.node[kind.idx()] = glyph;
    }

    #[inline]
    pub fn set_connection_glyph(&mut self, kind: ConnectionKind, glyph: char) {
        self.connection[kind.idx()] = glyph;
    }

    pub fn apply_overrides(
        &mut self,
        node_overrides: impl IntoIterator<Item = (NodeKind, char)>,
        connection_overrides: impl IntoIterator<Item = (ConnectionKind, char)>,
    ) {
        for (kind, glyph) in node_overrides {
            self.set_node_glyph(kind, glyph);
        }
        for (kind, glyph) in connection_overrides {
            self.set_connection_glyph(kind, glyph);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackCell {
    Node(NodeKind),
    Connection(ConnectionKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Renderable {
    pub x: usize,
    pub lane_id: Option<LaneId>,
    pub cell: TrackCell,
}

#[derive(Clone, Debug)]
pub struct RowPlan<'a, T> {
    pub node: &'a T,
    pub parent_availability: ParentAvailability,
    pub node_lane_col: usize,
    pub width: usize,
    pub operations: Vec<Renderable>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteReason {
    CarryLane,
    NodeLaneShift,
    MergeIntoNode,
    ParentOut,
    DanglingUp,
}

impl RouteReason {
    pub const fn as_snake(self) -> &'static str {
        match self {
            RouteReason::CarryLane => "carry_lane",
            RouteReason::NodeLaneShift => "node_lane_shift",
            RouteReason::MergeIntoNode => "merge_into_node",
            RouteReason::ParentOut => "parent_out",
            RouteReason::DanglingUp => "dangling_up",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MaskRoute {
    pub reason: RouteReason,
    pub lane_id: Option<LaneId>,
    pub from_column: usize,
    pub to_column: usize,
    pub include_up_segment: bool,
    pub include_down_segment: bool,
}

#[derive(Clone, Debug)]
pub struct ColumnComputation {
    pub column: usize,
    pub lane_id_above: Option<LaneId>,
    pub lane_id_below: Option<LaneId>,
    pub is_merge_column: bool,
    pub is_node_column: bool,
    pub mask: u8,
    pub connection_kind: ConnectionKind,
}

#[derive(Clone, Debug)]
pub struct StepDetails<'a, T> {
    pub plan: RowPlan<'a, T>,
    pub node_lane_id: LaneId,
    pub merge_columns: Vec<usize>,
    pub parent_lane_ids: Vec<LaneId>,
    pub lanes_above: Vec<Option<LaneId>>,
    pub lanes_below: Vec<Option<LaneId>>,
    pub routes: Vec<MaskRoute>,
    pub columns: Vec<ColumnComputation>,
}

#[derive(Clone, Debug)]
struct ActiveLane<Id> {
    lane_id: LaneId,
    target: Id,
}

#[derive(Clone, Debug, Default)]
struct BuildBelowScratch {
    parent_lane_ids: Vec<LaneId>,
    empty_cols_without_above_lane: Vec<usize>,
    empty_cols_with_above_lane: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
struct CollapseScratch {
    collapse_candidates: Vec<(usize, usize)>,
    horizontal_blocked_columns: Vec<bool>,
}

#[derive(Clone, Debug)]
struct LaneRow<Id>(Vec<Option<ActiveLane<Id>>>);

impl<Id> Default for LaneRow<Id> {
    #[inline]
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<Id> LaneRow<Id> {
    #[inline]
    fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn iter(&self) -> std::slice::Iter<'_, Option<ActiveLane<Id>>> {
        self.0.iter()
    }

    #[inline]
    fn get_mut(&mut self, column_index: usize) -> Option<&mut Option<ActiveLane<Id>>> {
        self.0.get_mut(column_index)
    }

    #[inline]
    fn push(&mut self, lane: Option<ActiveLane<Id>>) {
        self.0.push(lane);
    }

    #[inline]
    fn pop(&mut self) -> Option<Option<ActiveLane<Id>>> {
        self.0.pop()
    }

    #[inline]
    fn last(&self) -> Option<&Option<ActiveLane<Id>>> {
        self.0.last()
    }

    #[inline]
    fn trim_right(&mut self) {
        while matches!(self.last(), Some(None)) {
            self.pop();
        }
    }

    #[inline]
    fn first_free_col(&self) -> Option<usize> {
        self.0.iter().position(Option::is_none)
    }

    #[inline]
    fn lane_id_at(&self, col: usize) -> Option<LaneId> {
        self.0
            .get(col)
            .and_then(|opt| opt.as_ref().map(|l| l.lane_id))
    }

    fn find_target_matches_into(&self, target: &Id, merge_columns: &mut Vec<usize>) -> Option<usize>
    where
        Id: Eq,
    {
        merge_columns.clear();
        let mut first_match_column = None;

        for (column_index, lane) in self.0.iter().enumerate() {
            let Some(lane) = lane.as_ref() else { continue };
            if &lane.target != target {
                continue;
            }

            if first_match_column.is_none() {
                first_match_column = Some(column_index);
            } else {
                merge_columns.push(column_index);
            }
        }

        first_match_column
    }
}

impl<Id> std::ops::Index<usize> for LaneRow<Id> {
    type Output = Option<ActiveLane<Id>>;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<Id> std::ops::IndexMut<usize> for LaneRow<Id> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[derive(Clone, Debug, Default)]
struct LanePosIndex {
    lane_id_to_column: Vec<usize>,
    lane_slot_generation: Vec<u32>,
    active_generation: u32,
}

impl LanePosIndex {
    #[inline]
    fn clear(&mut self) {
        self.active_generation = self.active_generation.wrapping_add(1);
        if self.active_generation == 0 {
            self.lane_slot_generation.fill(0);
            self.active_generation = 1;
        }
    }

    #[inline]
    fn insert(&mut self, lane_id: LaneId, column: usize) {
        let lane_index = lane_id as usize;
        if lane_index >= self.lane_id_to_column.len() {
            let new_len = lane_index + 1;
            self.lane_id_to_column.resize(new_len, 0);
            self.lane_slot_generation.resize(new_len, 0);
        }
        self.lane_id_to_column[lane_index] = column;
        self.lane_slot_generation[lane_index] = self.active_generation;
    }

    #[inline]
    fn get(&self, lane_id: LaneId) -> Option<usize> {
        let lane_index = lane_id as usize;
        if lane_index < self.lane_slot_generation.len()
            && self.lane_slot_generation[lane_index] == self.active_generation
        {
            Some(self.lane_id_to_column[lane_index])
        } else {
            None
        }
    }

    fn rebuild_from_row<Id>(&mut self, lanes_below: &LaneRow<Id>) {
        self.clear();
        for (column, lane) in lanes_below.iter().enumerate() {
            if let Some(lane) = lane.as_ref() {
                self.insert(lane.lane_id, column);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GraphLayout<Id> {
    next_lane_id: LaneId,
    active_lanes_above: LaneRow<Id>,
    lane_ids_above: Vec<Option<LaneId>>,
    merged_columns: Vec<usize>,
    merged_column_flags: Vec<bool>,
    build_below_scratch: BuildBelowScratch,
    connection_masks: Vec<u8>,
    horizontal_diff: Vec<i32>,
    collapse_scratch: CollapseScratch,
    lane_positions_below: LanePosIndex,
}

#[derive(Clone, Debug)]
struct StepDetailsParts {
    node_lane_id: LaneId,
    merge_columns: Vec<usize>,
    parent_lane_ids: Vec<LaneId>,
    lanes_above: Vec<Option<LaneId>>,
    lanes_below: Vec<Option<LaneId>>,
    routes: Vec<MaskRoute>,
    columns: Vec<ColumnComputation>,
}

impl<Id> Default for GraphLayout<Id> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> GraphLayout<Id> {
    #[inline]
    pub fn new() -> Self {
        Self {
            next_lane_id: 1,
            active_lanes_above: LaneRow::new(),
            lane_ids_above: Vec::new(),
            merged_columns: Vec::new(),
            merged_column_flags: Vec::new(),
            build_below_scratch: BuildBelowScratch::default(),
            connection_masks: Vec::new(),
            horizontal_diff: Vec::new(),
            collapse_scratch: CollapseScratch::default(),
            lane_positions_below: LanePosIndex::default(),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.next_lane_id = 1;
        self.active_lanes_above.clear();
    }
}

impl<Id> GraphLayout<Id>
where
    Id: Clone + Eq + Hash,
{
    #[must_use]
    pub fn layout<'a, T>(&mut self, nodes: &'a [T]) -> Vec<RowPlan<'a, T>>
    where
        T: GraphNode<Id = Id>,
    {
        let mut plans = Vec::with_capacity(nodes.len());
        self.layout_with(nodes, |plan| plans.push(plan));
        plans
    }

    pub fn layout_with<'a, T, Emit>(&mut self, nodes: &'a [T], mut emit: Emit)
    where
        T: GraphNode<Id = Id>,
        Emit: FnMut(RowPlan<'a, T>),
    {
        self.layout_with_lanes_below(nodes, |plan, _lanes_below| emit(plan));
    }

    fn layout_with_lanes_below<'a, T, Emit>(&mut self, nodes: &'a [T], mut emit: Emit)
    where
        T: GraphNode<Id = Id>,
        Emit: FnMut(RowPlan<'a, T>, &LaneRow<Id>),
    {
        for_each_node(
            nodes,
            |graph_node, node_id, child_count, parent_availability| {
                let plan = self.layout_step(graph_node, node_id, child_count, parent_availability);
                emit(plan, &self.active_lanes_above);
            },
        );
    }

    #[must_use]
    pub fn layout_steps<'a, T>(&mut self, nodes: &'a [T]) -> Vec<StepDetails<'a, T>>
    where
        T: GraphNode<Id = Id>,
    {
        let mut rows = Vec::with_capacity(nodes.len());
        self.layout_with_steps(nodes, |row| rows.push(row));
        rows
    }

    pub fn layout_with_steps<'a, T, Emit>(&mut self, nodes: &'a [T], mut emit: Emit)
    where
        T: GraphNode<Id = Id>,
        Emit: FnMut(StepDetails<'a, T>),
    {
        for_each_node(
            nodes,
            |graph_node, node_id, child_count, parent_availability| {
                let row =
                    self.layout_step_details(graph_node, node_id, child_count, parent_availability);
                emit(row);
            },
        );
    }

    fn layout_step<'a, T>(
        &mut self,
        node: &'a T,
        node_id: T::Id,
        child_count: usize,
        parent_availability: ParentAvailability,
    ) -> RowPlan<'a, T>
    where
        T: GraphNode<Id = Id>,
    {
        let (plan, _debug_details) =
            self.layout_step_internal(node, node_id, child_count, parent_availability, false);
        plan
    }

    fn layout_step_details<'a, T>(
        &mut self,
        node: &'a T,
        node_id: T::Id,
        child_count: usize,
        parent_availability: ParentAvailability,
    ) -> StepDetails<'a, T>
    where
        T: GraphNode<Id = Id>,
    {
        let (plan, debug_details) =
            self.layout_step_internal(node, node_id, child_count, parent_availability, true);
        let debug_details =
            debug_details.expect("debug details must be present when include_debug=true");

        StepDetails {
            plan,
            node_lane_id: debug_details.node_lane_id,
            merge_columns: debug_details.merge_columns,
            parent_lane_ids: debug_details.parent_lane_ids,
            lanes_above: debug_details.lanes_above,
            lanes_below: debug_details.lanes_below,
            routes: debug_details.routes,
            columns: debug_details.columns,
        }
    }

    fn layout_step_internal<'a, T>(
        &mut self,
        node: &'a T,
        node_id: T::Id,
        child_count: usize,
        parent_availability: ParentAvailability,
        include_debug: bool,
    ) -> (RowPlan<'a, T>, Option<StepDetailsParts>)
    where
        T: GraphNode<Id = Id>,
    {
        let mut lanes_above = std::mem::take(&mut self.active_lanes_above);
        lanes_above.trim_right();

        let existing_node_column =
            lanes_above.find_target_matches_into(&node_id, &mut self.merged_columns);
        make_merge_flags_into(
            &mut self.merged_column_flags,
            &self.merged_columns,
            lanes_above.len(),
        );
        let (node_lane_id, mut node_column) = ensure_node_lane(
            &mut self.next_lane_id,
            &mut lanes_above,
            &node_id,
            existing_node_column,
        );
        let node_lane_exists_above = existing_node_column.is_some();

        debug_assert!(lanes_above.lane_id_at(node_column) == Some(node_lane_id));

        let mut lane_ids_above = std::mem::take(&mut self.lane_ids_above);
        snapshot_lane_ids(&lanes_above, &mut lane_ids_above);

        let mut lanes_below = lanes_above;
        build_below_with_merge_flags(
            &mut self.next_lane_id,
            &mut lanes_below,
            BuildBelowParams {
                lane_ids_above: &lane_ids_above,
                node,
                node_column,
                merged_columns: &self.merged_columns,
                merged_column_flags: &self.merged_column_flags,
                node_lane_id,
            },
            &mut self.build_below_scratch,
        );
        lanes_below.trim_right();
        self.lane_positions_below.rebuild_from_row(&lanes_below);
        compact_lanes_direct(
            &MaskParams {
                lane_ids_above: &lane_ids_above,
                node_column,
                node_lane_id,
                merged_columns: &self.merged_columns,
                merged_column_flags: &self.merged_column_flags,
                parent_lane_ids: &self.build_below_scratch.parent_lane_ids,
                lane_width: lane_ids_above.len().max(lanes_below.len()),
                node_lane_exists_above,
                lane_positions_below: &self.lane_positions_below,
            },
            &mut lanes_below,
            &mut self.connection_masks,
            &mut self.horizontal_diff,
            &mut self.collapse_scratch,
        );
        lanes_below.trim_right();
        self.lane_positions_below.rebuild_from_row(&lanes_below);

        if let Some(current_node_column) = self.lane_positions_below.get(node_lane_id) {
            node_column = current_node_column;
        }

        let lane_width = lane_ids_above.len().max(lanes_below.len());
        let params = MaskParams {
            lane_ids_above: &lane_ids_above,
            node_column,
            node_lane_id,
            merged_columns: &self.merged_columns,
            merged_column_flags: &self.merged_column_flags,
            parent_lane_ids: &self.build_below_scratch.parent_lane_ids,
            lane_width,
            node_lane_exists_above,
            lane_positions_below: &self.lane_positions_below,
        };

        let mut routes = Vec::new();
        if include_debug {
            compute_masks_with_routes(
                &params,
                true,
                &mut self.connection_masks,
                &mut self.horizontal_diff,
                &mut routes,
            );
        } else {
            compute_masks_with_merge_flags_and_pos(
                &params,
                true,
                &mut self.connection_masks,
                &mut self.horizontal_diff,
            );
        }

        let node_kind = classify_node(node, child_count, parent_availability);
        let operations = build_operations(
            lane_width,
            node_column,
            node_kind,
            &lane_ids_above,
            &lanes_below,
            &self.connection_masks[..lane_width],
        );

        let debug_details = if include_debug {
            let mut columns = Vec::with_capacity(lane_width);
            for column in 0..lane_width {
                columns.push(ColumnComputation {
                    column,
                    lane_id_above: lane_id_at_snapshot(&lane_ids_above, column),
                    lane_id_below: lanes_below.lane_id_at(column),
                    is_merge_column: is_merge_col(&self.merged_column_flags, column),
                    is_node_column: column == node_column,
                    mask: self.connection_masks[column],
                    connection_kind: Mask::from_mask(self.connection_masks[column]),
                });
            }

            let mut lanes_above_snapshot = lane_ids_above.clone();
            lanes_above_snapshot.resize(lane_width, None);
            let lanes_below_snapshot = (0..lane_width)
                .map(|column| lanes_below.lane_id_at(column))
                .collect::<Vec<_>>();

            Some(StepDetailsParts {
                node_lane_id,
                merge_columns: self.merged_columns.clone(),
                parent_lane_ids: self.build_below_scratch.parent_lane_ids.clone(),
                lanes_above: lanes_above_snapshot,
                lanes_below: lanes_below_snapshot,
                routes,
                columns,
            })
        } else {
            None
        };

        let width = if lane_width == 0 {
            0
        } else {
            lane_width.saturating_mul(2).saturating_sub(1)
        };
        let plan = RowPlan {
            node,
            parent_availability,
            node_lane_col: node_column,
            width,
            operations,
        };
        self.active_lanes_above = lanes_below;
        self.lane_ids_above = lane_ids_above;
        (plan, debug_details)
    }
}

fn for_each_node<'a, T, Id, Emit>(nodes: &'a [T], mut emit: Emit)
where
    T: GraphNode<Id = Id>,
    Id: Clone + Eq + Hash,
    Emit: FnMut(&'a T, Id, usize, ParentAvailability),
{
    let mut child_count_by_id = HashMap::with_capacity(nodes.len());
    let mut ordered_node_ids = Vec::with_capacity(nodes.len());

    for graph_node in nodes {
        let node_id = graph_node.id();
        child_count_by_id.entry(node_id.clone()).or_default();
        ordered_node_ids.push(node_id);
    }

    for graph_node in nodes {
        for parent_id in graph_node.parents() {
            if let Some(child_count) = child_count_by_id.get_mut(parent_id) {
                *child_count += 1;
            }
        }
    }

    for (graph_node, node_id) in nodes.iter().zip(ordered_node_ids) {
        let child_count = child_count_by_id.get(&node_id).copied().unwrap_or(0);
        let mut present_parent_count = 0usize;
        let mut missing_parent_count = 0usize;

        for parent_id in graph_node.parents() {
            if child_count_by_id.contains_key(parent_id) {
                present_parent_count += 1;
            } else {
                missing_parent_count += 1;
            }
        }

        emit(
            graph_node,
            node_id,
            child_count,
            ParentAvailability::new(present_parent_count, missing_parent_count),
        );
    }
}

#[inline]
fn next_lane_id(next_lane_id_counter: &mut LaneId) -> LaneId {
    let assigned_lane_id = *next_lane_id_counter;
    *next_lane_id_counter = assigned_lane_id.saturating_add(1);
    assigned_lane_id
}

fn ensure_node_lane<Id: Clone>(
    next_lane_id_counter: &mut LaneId,
    lane_columns: &mut LaneRow<Id>,
    node_id: &Id,
    existing_column: Option<usize>,
) -> (LaneId, usize) {
    if let Some(existing_column) = existing_column {
        let existing_lane_id = lane_columns[existing_column].as_ref().unwrap().lane_id;
        return (existing_lane_id, existing_column);
    }

    let assigned_lane_id = next_lane_id(next_lane_id_counter);

    if let Some(free_column) = lane_columns.first_free_col() {
        lane_columns[free_column] = Some(ActiveLane {
            lane_id: assigned_lane_id,
            target: node_id.clone(),
        });
        return (assigned_lane_id, free_column);
    }

    lane_columns.push(Some(ActiveLane {
        lane_id: assigned_lane_id,
        target: node_id.clone(),
    }));
    (assigned_lane_id, lane_columns.len() - 1)
}

#[inline]
fn snapshot_lane_ids<Id>(lane_row: &LaneRow<Id>, lane_ids: &mut Vec<Option<LaneId>>) {
    lane_ids.clear();
    lane_ids.reserve(lane_row.len());
    for lane in lane_row.iter() {
        lane_ids.push(lane.as_ref().map(|active_lane| active_lane.lane_id));
    }
}

#[inline]
fn lane_id_at_snapshot(lane_ids: &[Option<LaneId>], column: usize) -> Option<LaneId> {
    lane_ids.get(column).copied().flatten()
}

#[derive(Clone, Copy)]
struct Mask;

impl Mask {
    const ALL: u8 = Self::UP | Self::RIGHT | Self::DOWN | Self::LEFT;
    const CORNER_DOWN_LEFT: u8 = Self::DOWN | Self::LEFT;
    const CORNER_DOWN_RIGHT: u8 = Self::DOWN | Self::RIGHT;
    const CORNER_UP_LEFT: u8 = Self::UP | Self::LEFT;
    const CORNER_UP_RIGHT: u8 = Self::UP | Self::RIGHT;
    const DOWN: u8 = 1 << 2;
    const HORIZONTAL: u8 = Self::LEFT | Self::RIGHT;
    const LEFT: u8 = 1 << 3;
    const RIGHT: u8 = 1 << 1;
    const TEE_DOWN: u8 = Self::LEFT | Self::RIGHT | Self::DOWN;
    const TEE_LEFT: u8 = Self::UP | Self::DOWN | Self::LEFT;
    const TEE_RIGHT: u8 = Self::UP | Self::DOWN | Self::RIGHT;
    const TEE_UP: u8 = Self::LEFT | Self::RIGHT | Self::UP;
    const UP: u8 = 1 << 0;
    const VERTICAL: u8 = Self::UP | Self::DOWN;

    fn from_mask(mask: u8) -> ConnectionKind {
        match mask & Self::ALL {
            0 => ConnectionKind::Empty,

            Self::LEFT => ConnectionKind::EndLeft,
            Self::RIGHT => ConnectionKind::EndRight,
            Self::UP => ConnectionKind::EndUp,
            Self::DOWN => ConnectionKind::EndDown,

            Self::HORIZONTAL => ConnectionKind::Horizontal,
            Self::VERTICAL => ConnectionKind::Vertical,

            Self::CORNER_UP_LEFT => ConnectionKind::CornerUpLeft,
            Self::CORNER_UP_RIGHT => ConnectionKind::CornerUpRight,
            Self::CORNER_DOWN_RIGHT => ConnectionKind::CornerDownRight,
            Self::CORNER_DOWN_LEFT => ConnectionKind::CornerDownLeft,

            Self::TEE_LEFT => ConnectionKind::TeeLeft,
            Self::TEE_RIGHT => ConnectionKind::TeeRight,
            Self::TEE_UP => ConnectionKind::TeeUp,
            Self::TEE_DOWN => ConnectionKind::TeeDown,

            Self::ALL => ConnectionKind::CrossOver,

            _ => unreachable!("invalid connection mask: {mask:#b}"),
        }
    }
}

#[inline]
fn add_route_diff(
    connection_masks: &mut [u8],
    horizontal_diff: &mut [i32],
    from_column: usize,
    to_column: usize,
    include_up_segment: bool,
    include_down_segment: bool,
) {
    if from_column == to_column {
        if include_up_segment {
            connection_masks[from_column] |= Mask::UP;
        }
        if include_down_segment {
            connection_masks[from_column] |= Mask::DOWN;
        }
        return;
    }

    if to_column > from_column {
        connection_masks[from_column] |= Mask::RIGHT;
        connection_masks[to_column] |= Mask::LEFT;
    } else {
        connection_masks[from_column] |= Mask::LEFT;
        connection_masks[to_column] |= Mask::RIGHT;
    }

    if include_up_segment {
        connection_masks[from_column] |= Mask::UP;
    }
    if include_down_segment {
        connection_masks[to_column] |= Mask::DOWN;
    }

    let left_column = from_column.min(to_column);
    let right_column = from_column.max(to_column);
    if right_column > left_column + 1 {
        horizontal_diff[left_column + 1] += 1;
        horizontal_diff[right_column] -= 1;
    }
}

#[inline]
fn make_merge_flags_into(
    merged_column_flags: &mut Vec<bool>,
    merged_columns: &[usize],
    lane_count: usize,
) {
    merged_column_flags.resize(lane_count, false);
    merged_column_flags[..lane_count].fill(false);
    for &merged_column in merged_columns {
        if merged_column < lane_count {
            merged_column_flags[merged_column] = true;
        }
    }
}

#[inline]
fn is_merge_col(merged_column_flags: &[bool], column_index: usize) -> bool {
    merged_column_flags
        .get(column_index)
        .copied()
        .unwrap_or(false)
}

struct MaskParams<'a> {
    lane_ids_above: &'a [Option<LaneId>],
    node_column: usize,
    node_lane_id: LaneId,
    merged_columns: &'a [usize],
    merged_column_flags: &'a [bool],
    parent_lane_ids: &'a [LaneId],
    lane_width: usize,
    node_lane_exists_above: bool,
    lane_positions_below: &'a LanePosIndex,
}

struct BuildBelowParams<'a, T> {
    lane_ids_above: &'a [Option<LaneId>],
    node: &'a T,
    node_column: usize,
    merged_columns: &'a [usize],
    merged_column_flags: &'a [bool],
    node_lane_id: LaneId,
}

#[inline]
fn ensure_mask_buffers(
    connection_masks: &mut Vec<u8>,
    horizontal_diff: &mut Vec<i32>,
    lane_width: usize,
) {
    if connection_masks.len() < lane_width {
        connection_masks.resize(lane_width, 0);
    }
    connection_masks[..lane_width].fill(0);

    let horizontal_diff_len = lane_width.saturating_add(1);
    if horizontal_diff.len() < horizontal_diff_len {
        horizontal_diff.resize(horizontal_diff_len, 0);
    }
    horizontal_diff[..horizontal_diff_len].fill(0);
}

fn compute_masks_with_merge_flags_and_pos(
    params: &MaskParams<'_>,
    include_vertical_segments: bool,
    connection_masks: &mut Vec<u8>,
    horizontal_diff: &mut Vec<i32>,
) {
    ensure_mask_buffers(connection_masks, horizontal_diff, params.lane_width);
    let connection_masks = &mut connection_masks[..params.lane_width];
    let horizontal_diff = &mut horizontal_diff[..params.lane_width.saturating_add(1)];

    for (source_column, source_lane_id) in params.lane_ids_above.iter().enumerate() {
        let Some(source_lane_id) = *source_lane_id else {
            continue;
        };

        if is_merge_col(params.merged_column_flags, source_column) {
            continue;
        }

        if source_lane_id == params.node_lane_id {
            if params.node_lane_exists_above
                && source_column != params.node_column
                && params.lane_positions_below.get(source_lane_id).is_some()
            {
                add_route_diff(
                    connection_masks,
                    horizontal_diff,
                    source_column,
                    params.node_column,
                    include_vertical_segments,
                    false,
                );
            }
            continue;
        }

        if let Some(destination_column) = params.lane_positions_below.get(source_lane_id) {
            add_route_diff(
                connection_masks,
                horizontal_diff,
                source_column,
                destination_column,
                include_vertical_segments,
                include_vertical_segments,
            );
        } else if include_vertical_segments {
            connection_masks[source_column] |= Mask::UP;
        }
    }

    for &source_column in params.merged_columns {
        add_route_diff(
            connection_masks,
            horizontal_diff,
            source_column,
            params.node_column,
            include_vertical_segments,
            false,
        );
    }

    for &parent_lane_id in params.parent_lane_ids {
        if let Some(destination_column) = params.lane_positions_below.get(parent_lane_id) {
            add_route_diff(
                connection_masks,
                horizontal_diff,
                params.node_column,
                destination_column,
                false,
                include_vertical_segments,
            );
        }
    }

    let mut active_horizontal_span_count = 0i32;
    for column in 0..params.lane_width {
        active_horizontal_span_count += horizontal_diff[column];
        if active_horizontal_span_count != 0 {
            connection_masks[column] |= Mask::LEFT | Mask::RIGHT;
        }
    }
}

fn compute_masks_with_routes(
    params: &MaskParams<'_>,
    include_up_segment: bool,
    connection_masks: &mut Vec<u8>,
    horizontal_diff: &mut Vec<i32>,
    routes: &mut Vec<MaskRoute>,
) {
    routes.clear();
    ensure_mask_buffers(connection_masks, horizontal_diff, params.lane_width);
    let connection_masks = &mut connection_masks[..params.lane_width];
    let horizontal_diff = &mut horizontal_diff[..params.lane_width.saturating_add(1)];

    for (source_column, source_lane_id) in params.lane_ids_above.iter().enumerate() {
        let Some(source_lane_id) = *source_lane_id else {
            continue;
        };

        if is_merge_col(params.merged_column_flags, source_column) {
            continue;
        }

        if source_lane_id == params.node_lane_id {
            if params.node_lane_exists_above
                && source_column != params.node_column
                && params.lane_positions_below.get(source_lane_id).is_some()
            {
                routes.push(MaskRoute {
                    reason: RouteReason::NodeLaneShift,
                    lane_id: Some(source_lane_id),
                    from_column: source_column,
                    to_column: params.node_column,
                    include_up_segment,
                    include_down_segment: false,
                });
                add_route_diff(
                    connection_masks,
                    horizontal_diff,
                    source_column,
                    params.node_column,
                    include_up_segment,
                    false,
                );
            }
            continue;
        }

        if let Some(destination_column) = params.lane_positions_below.get(source_lane_id) {
            routes.push(MaskRoute {
                reason: RouteReason::CarryLane,
                lane_id: Some(source_lane_id),
                from_column: source_column,
                to_column: destination_column,
                include_up_segment,
                include_down_segment: include_up_segment,
            });
            add_route_diff(
                connection_masks,
                horizontal_diff,
                source_column,
                destination_column,
                include_up_segment,
                include_up_segment,
            );
        } else if include_up_segment {
            routes.push(MaskRoute {
                reason: RouteReason::DanglingUp,
                lane_id: Some(source_lane_id),
                from_column: source_column,
                to_column: source_column,
                include_up_segment: true,
                include_down_segment: false,
            });
            add_route_diff(
                connection_masks,
                horizontal_diff,
                source_column,
                source_column,
                true,
                false,
            );
        }
    }

    for &source_column in params.merged_columns {
        routes.push(MaskRoute {
            reason: RouteReason::MergeIntoNode,
            lane_id: None,
            from_column: source_column,
            to_column: params.node_column,
            include_up_segment,
            include_down_segment: false,
        });
        add_route_diff(
            connection_masks,
            horizontal_diff,
            source_column,
            params.node_column,
            include_up_segment,
            false,
        );
    }

    for &parent_lane_id in params.parent_lane_ids {
        if let Some(destination_column) = params.lane_positions_below.get(parent_lane_id) {
            routes.push(MaskRoute {
                reason: RouteReason::ParentOut,
                lane_id: Some(parent_lane_id),
                from_column: params.node_column,
                to_column: destination_column,
                include_up_segment: false,
                include_down_segment: include_up_segment,
            });
            add_route_diff(
                connection_masks,
                horizontal_diff,
                params.node_column,
                destination_column,
                false,
                include_up_segment,
            );
        }
    }

    let mut active_horizontal_span_count = 0i32;
    for column in 0..params.lane_width {
        active_horizontal_span_count += horizontal_diff[column];
        if active_horizontal_span_count != 0 {
            connection_masks[column] |= Mask::LEFT | Mask::RIGHT;
        }
    }
}

fn classify_node<T: GraphNode>(
    node: &T,
    child_count: usize,
    parent_availability: ParentAvailability,
) -> NodeKind {
    match parent_availability.missing_parent_state() {
        MissingParentState::All => NodeKind::Orphan,
        MissingParentState::Some => {
            if child_count == 0 {
                NodeKind::MergeLeafTruncated
            } else {
                NodeKind::MergeTruncated
            }
        }
        MissingParentState::None if node.parents().is_empty() => NodeKind::Initial,
        MissingParentState::None if child_count == 0 && node.parents().len() >= 2 => {
            NodeKind::MergeLeaf
        }
        MissingParentState::None if child_count == 0 => NodeKind::NodeLeaf,
        MissingParentState::None if node.parents().len() >= 2 => NodeKind::Merge,
        MissingParentState::None => NodeKind::Node,
    }
}

fn build_operations<Id>(
    lane_width: usize,
    node_column: usize,
    node_kind: NodeKind,
    lane_ids_above: &[Option<LaneId>],
    lanes_below: &LaneRow<Id>,
    connection_masks: &[u8],
) -> Vec<Renderable> {
    let mut operations = Vec::with_capacity(lane_width.saturating_mul(2).saturating_sub(1));

    let mut x_position = 0usize;
    for column_index in 0..lane_width {
        let lane_id_above = lane_id_at_snapshot(lane_ids_above, column_index);
        let lane_id_below = lanes_below.lane_id_at(column_index);

        let lane_id = match (lane_id_above, lane_id_below) {
            (Some(above_lane_id), Some(below_lane_id)) if above_lane_id == below_lane_id => {
                Some(above_lane_id)
            }
            (Some(above_lane_id), None) => Some(above_lane_id),
            (None, Some(below_lane_id)) => Some(below_lane_id),
            _ => None,
        };

        let cell = if column_index == node_column {
            TrackCell::Node(node_kind)
        } else {
            TrackCell::Connection(Mask::from_mask(connection_masks[column_index]))
        };

        let inter_column_connection_kind = if column_index + 1 < lane_width
            && (((connection_masks[column_index] & Mask::RIGHT) != 0)
                || ((connection_masks[column_index + 1] & Mask::LEFT) != 0))
        {
            ConnectionKind::Horizontal
        } else {
            ConnectionKind::Empty
        };

        operations.push(Renderable {
            x: x_position,
            lane_id,
            cell,
        });
        x_position += 1;

        if column_index < lane_width.saturating_sub(1) {
            let cell = TrackCell::Connection(inter_column_connection_kind);
            operations.push(Renderable {
                x: x_position,
                lane_id,
                cell,
            });
            x_position += 1;
        }
    }

    operations
}

#[inline]
fn collect_collapse_candidates_into<Id>(
    lane_ids_above: &[Option<LaneId>],
    lanes_below: &LaneRow<Id>,
    collapse_candidates: &mut Vec<(usize, usize)>,
) {
    collapse_candidates.clear();

    let lane_count = lanes_below.len();
    let mut gap_start = None;

    for column in 0..lane_count {
        let above_lane_id = lane_id_at_snapshot(lane_ids_above, column);
        let below_lane_id = lanes_below[column].as_ref().map(|lane| lane.lane_id);

        if above_lane_id.is_none() && below_lane_id.is_none() {
            if gap_start.is_none() {
                gap_start = Some(column);
            }
            continue;
        }

        if let Some(destination_column) = gap_start.take()
            && above_lane_id.is_some()
            && above_lane_id == below_lane_id
        {
            collapse_candidates.push((column, destination_column));
        }
    }
}

#[inline]
fn mark_horizontal_blockers(blocked_columns: &mut [bool], from_column: usize, to_column: usize) {
    for blocked_column in blocked_columns
        .iter_mut()
        .take(from_column.max(to_column) + 1)
        .skip(from_column.min(to_column))
    {
        *blocked_column = true;
    }
}

fn compact_lanes_direct<Id>(
    initial_mask_params: &MaskParams<'_>,
    lanes_below: &mut LaneRow<Id>,
    connection_masks: &mut Vec<u8>,
    horizontal_diff: &mut Vec<i32>,
    scratch: &mut CollapseScratch,
) {
    let CollapseScratch {
        collapse_candidates,
        horizontal_blocked_columns,
    } = scratch;
    let lane_width = initial_mask_params.lane_width;
    if lane_width < 2 {
        return;
    }

    compute_masks_with_merge_flags_and_pos(
        initial_mask_params,
        false,
        connection_masks,
        horizontal_diff,
    );

    if horizontal_blocked_columns.len() < lane_width {
        horizontal_blocked_columns.resize(lane_width, false);
    }
    horizontal_blocked_columns[..lane_width].fill(false);
    for column in 0..lane_width {
        horizontal_blocked_columns[column] =
            (connection_masks[column] & (Mask::LEFT | Mask::RIGHT)) != 0;
    }

    collect_collapse_candidates_into(
        initial_mask_params.lane_ids_above,
        lanes_below,
        collapse_candidates,
    );

    // Horizontal blockers only ever grow during collapse, so processing the original movable
    // sources from right to left reaches the same fixed point as the old one-move-at-a-time loop.
    for &(source_column, destination_column) in collapse_candidates.iter().rev() {
        if horizontal_blocked_columns[destination_column] {
            continue;
        }
        if lanes_below[destination_column].is_some() {
            continue;
        }

        let Some(source_lane) = lanes_below[source_column].as_ref() else {
            continue;
        };
        if lane_id_at_snapshot(initial_mask_params.lane_ids_above, source_column)
            != Some(source_lane.lane_id)
        {
            continue;
        }

        let moved_lane_id = source_lane.lane_id;
        lanes_below[destination_column] = lanes_below[source_column].take();
        mark_horizontal_blockers(
            &mut horizontal_blocked_columns[..lane_width],
            source_column,
            destination_column,
        );

        if moved_lane_id == initial_mask_params.node_lane_id {
            debug_assert!(
                initial_mask_params.lane_ids_above[source_column]
                    == Some(initial_mask_params.node_lane_id)
            );
        }
    }
}

fn build_below_with_merge_flags<T>(
    next_lane_id_counter: &mut LaneId,
    lanes_below: &mut LaneRow<T::Id>,
    params: BuildBelowParams<'_, T>,
    scratch: &mut BuildBelowScratch,
) where
    T: GraphNode,
    T::Id: Clone,
{
    let BuildBelowScratch {
        parent_lane_ids,
        empty_cols_without_above_lane,
        empty_cols_with_above_lane,
    } = scratch;

    for &merged_column in params.merged_columns {
        lanes_below[merged_column] = None;
    }

    parent_lane_ids.clear();
    empty_cols_without_above_lane.clear();
    empty_cols_with_above_lane.clear();

    match params.node.parents().split_first() {
        None => {
            lanes_below[params.node_column] = None;
        }
        Some((first_parent, extra_parents)) => {
            if let Some(lane) = lanes_below
                .get_mut(params.node_column)
                .and_then(|lane_option| lane_option.as_mut())
            {
                lane.target = first_parent.clone();
                parent_lane_ids.push(params.node_lane_id);
            }

            let scan_start_column = params.node_column.saturating_add(1);
            if scan_start_column < lanes_below.len() {
                for column_index in scan_start_column..lanes_below.len() {
                    if column_index == params.node_column
                        || is_merge_col(params.merged_column_flags, column_index)
                    {
                        continue;
                    }
                    if lanes_below[column_index].is_none() {
                        if lane_id_at_snapshot(params.lane_ids_above, column_index).is_none() {
                            empty_cols_without_above_lane.push(column_index);
                        } else {
                            empty_cols_with_above_lane.push(column_index);
                        }
                    }
                }
            }

            let mut without_above_idx = 0usize;
            let mut with_above_idx = 0usize;

            for extra_parent_id in extra_parents {
                let new_lane_id = next_lane_id(next_lane_id_counter);
                parent_lane_ids.push(new_lane_id);

                let mut pending_lane = Some(ActiveLane {
                    lane_id: new_lane_id,
                    target: extra_parent_id.clone(),
                });

                while without_above_idx < empty_cols_without_above_lane.len()
                    && lanes_below[empty_cols_without_above_lane[without_above_idx]].is_some()
                {
                    without_above_idx += 1;
                }
                if without_above_idx < empty_cols_without_above_lane.len() {
                    lanes_below[empty_cols_without_above_lane[without_above_idx]] =
                        pending_lane.take();
                    without_above_idx += 1;
                }

                if pending_lane.is_some() {
                    while with_above_idx < empty_cols_with_above_lane.len()
                        && lanes_below[empty_cols_with_above_lane[with_above_idx]].is_some()
                    {
                        with_above_idx += 1;
                    }
                    if with_above_idx < empty_cols_with_above_lane.len() {
                        lanes_below[empty_cols_with_above_lane[with_above_idx]] =
                            pending_lane.take();
                        with_above_idx += 1;
                    }
                }

                if let Some(remaining_lane) = pending_lane.take() {
                    lanes_below.push(Some(remaining_lane));
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderConfig {
    pub glyphs: Glyphs,
    pub render_terminal_lanes: bool,
    pub minimum_inter_node_rows: usize,
}

impl RenderConfig {
    #[inline]
    pub fn glyph_for_node(&self, kind: NodeKind) -> char {
        self.glyphs.node[kind.idx()]
    }

    #[inline]
    pub fn glyph_for_connection(&self, kind: ConnectionKind) -> char {
        self.glyphs.connection[kind.idx()]
    }

    #[inline]
    pub fn set_node_glyph(&mut self, kind: NodeKind, glyph: char) {
        self.glyphs.node[kind.idx()] = glyph;
    }

    #[inline]
    pub fn set_connection_glyph(&mut self, kind: ConnectionKind, glyph: char) {
        self.glyphs.connection[kind.idx()] = glyph;
    }

    #[inline]
    pub fn set_render_terminal_lanes(&mut self, enabled: bool) {
        self.render_terminal_lanes = enabled;
    }

    #[inline]
    pub fn set_minimum_inter_node_rows(&mut self, row_count: usize) {
        self.minimum_inter_node_rows = row_count;
    }
}

#[derive(Clone, Debug)]
pub struct GraphRenderer {
    config: RenderConfig,
    layout: GraphLayout<String>,
    last_fingerprint: Option<u64>,
    rendered: String,
}

impl Default for GraphRenderer {
    fn default() -> Self {
        Self::new(RenderConfig::default())
    }
}

impl GraphRenderer {
    #[inline]
    pub fn new(config: RenderConfig) -> Self {
        Self {
            config,
            layout: GraphLayout::new(),
            last_fingerprint: None,
            rendered: String::new(),
        }
    }

    #[inline]
    pub fn config(&self) -> &RenderConfig {
        &self.config
    }

    #[inline]
    pub fn reset(&mut self) {
        self.layout.reset();
        self.last_fingerprint = None;
        self.rendered.clear();
    }

    #[inline]
    pub fn rendered(&self) -> &str {
        &self.rendered
    }

    pub fn render_if_changed<T>(&mut self, nodes: &[T]) -> bool
    where
        T: GraphNode<Id = std::string::String>,
        T::Id: Clone + Eq,
    {
        self.render_if_changed_with_collected_content(nodes, None)
    }

    /// Renders only when graph topology or callback-provided content has changed.
    ///
    /// The callback is evaluated once per node on every render attempt. Borrowed content remains
    /// borrowed for the duration of the call; owned strings are also accepted.
    pub fn render_if_changed_with_content<'a, T, Content, ContentFor>(
        &mut self,
        nodes: &'a [T],
        mut content_for: ContentFor,
    ) -> bool
    where
        T: GraphNode<Id = std::string::String>,
        T::Id: Clone + Eq,
        Content: Into<Cow<'a, str>>,
        ContentFor: FnMut(&'a T) -> Content,
    {
        let content_by_node = nodes
            .iter()
            .map(|node| content_for(node).into())
            .collect::<Vec<Cow<'a, str>>>();
        self.render_if_changed_with_collected_content(nodes, Some(&content_by_node))
    }

    fn render_if_changed_with_collected_content<T>(
        &mut self,
        nodes: &[T],
        content_by_node: Option<&[Cow<'_, str>]>,
    ) -> bool
    where
        T: GraphNode<Id = std::string::String>,
        T::Id: Clone + Eq,
    {
        debug_assert!(content_by_node.is_none_or(|content| content.len() == nodes.len()));

        let fingerprint = render_fingerprint(nodes, content_by_node, &self.config);
        if self.last_fingerprint == Some(fingerprint) {
            return false;
        }

        self.layout.reset();
        self.rendered.clear();

        let config = &self.config;
        let layout = &mut self.layout;
        let rendered_output = &mut self.rendered;
        let mut node_index = 0usize;

        layout.layout_with_lanes_below(nodes, |plan, lanes_below| {
            let content = content_by_node
                .and_then(|content| content.get(node_index))
                .map_or("", |content| content.as_ref());
            let mut content_lines = content.split('\n');
            let first_content_line = content_lines.next().unwrap_or_default();

            render_plan_with_config(config, &plan, rendered_output);
            append_content_line(rendered_output, first_content_line);
            rendered_output.push('\n');

            let mut content_continuation_count = 0usize;
            for content_line in content_lines {
                let content_line_has_text = !normalized_content_line(content_line).is_empty();
                let minimum_width = if content_line_has_text { plan.width } else { 0 };
                let rendered = render_lane_row_prefix_with_config(
                    config,
                    lanes_below,
                    ConnectionKind::Vertical,
                    minimum_width,
                    rendered_output,
                );
                debug_assert!(rendered || !content_line_has_text);
                append_content_line(rendered_output, content_line);
                rendered_output.push('\n');
                content_continuation_count += 1;
            }

            if node_index + 1 < nodes.len() {
                let remaining_row_count = config
                    .minimum_inter_node_rows
                    .saturating_sub(content_continuation_count);
                for _ in 0..remaining_row_count {
                    if !render_lane_row_with_config(
                        config,
                        lanes_below,
                        ConnectionKind::Vertical,
                        rendered_output,
                    ) {
                        break;
                    }
                }
            }

            node_index += 1;
        });
        if config.render_terminal_lanes {
            render_terminal_lanes_with_config(
                config,
                &self.layout.active_lanes_above,
                rendered_output,
            );
        }

        self.last_fingerprint = Some(fingerprint);
        true
    }

    /// Renders the complete graph without node content.
    #[must_use]
    pub fn render_to_string(&mut self, nodes: &[Node]) -> String {
        self.render_if_changed(nodes);
        self.rendered.clone()
    }

    /// Renders the complete graph with content beside each node.
    ///
    /// The first content line is rendered on the node row. Every newline creates one vertical
    /// continuation row, including blank and trailing lines.
    #[must_use]
    pub fn render_to_string_with_content<'a, T, Content, ContentFor>(
        &mut self,
        nodes: &'a [T],
        content_for: ContentFor,
    ) -> String
    where
        T: GraphNode<Id = std::string::String>,
        T::Id: Clone + Eq,
        Content: Into<Cow<'a, str>>,
        ContentFor: FnMut(&'a T) -> Content,
    {
        self.render_if_changed_with_content(nodes, content_for);
        self.rendered.clone()
    }

    /// Renders only the graph cells for one node row.
    ///
    /// Node content and inter-node rows require whole-graph context and are emitted by the
    /// whole-graph rendering methods instead.
    #[inline]
    pub fn render_plan_into(&self, plan: &RowPlan<'_, Node>, output: &mut String) {
        render_plan_with_config(&self.config, plan, output);
    }
}

fn render_plan_with_config<T: GraphNode>(
    config: &RenderConfig,
    plan: &RowPlan<'_, T>,
    output: &mut String,
) {
    output.reserve(plan.operations.len());
    for op in &plan.operations {
        match op.cell {
            TrackCell::Node(kind) => output.push(config.glyph_for_node(kind)),
            TrackCell::Connection(kind) => output.push(config.glyph_for_connection(kind)),
        }
    }
}

fn normalized_content_line(content_line: &str) -> &str {
    content_line.strip_suffix('\r').unwrap_or(content_line)
}

fn append_content_line(output: &mut String, content_line: &str) {
    let content_line = normalized_content_line(content_line);
    if content_line.is_empty() {
        return;
    }

    output.push(' ');
    output.push_str(content_line);
}

fn render_terminal_lanes_with_config<Id>(
    config: &RenderConfig,
    active_lanes: &LaneRow<Id>,
    output: &mut String,
) {
    render_lane_row_with_config(config, active_lanes, ConnectionKind::EndUp, output);
}

fn render_lane_row_with_config<Id>(
    config: &RenderConfig,
    active_lanes: &LaneRow<Id>,
    active_connection_kind: ConnectionKind,
    output: &mut String,
) -> bool {
    if !render_lane_row_prefix_with_config(config, active_lanes, active_connection_kind, 0, output)
    {
        return false;
    }

    output.push('\n');
    true
}

fn render_lane_row_prefix_with_config<Id>(
    config: &RenderConfig,
    active_lanes: &LaneRow<Id>,
    active_connection_kind: ConnectionKind,
    minimum_width: usize,
    output: &mut String,
) -> bool {
    let active_lane_width = active_lanes
        .iter()
        .rposition(Option::is_some)
        .map_or(0, |last_active_column| last_active_column + 1);
    let minimum_lane_width = minimum_width.saturating_add(1) / 2;
    let lane_width = active_lane_width.max(minimum_lane_width);
    if lane_width == 0 {
        return false;
    }

    output.reserve(lane_width.saturating_mul(2));

    for column_index in 0..lane_width {
        let connection_kind = if active_lanes.lane_id_at(column_index).is_some() {
            active_connection_kind
        } else {
            ConnectionKind::Empty
        };
        output.push(config.glyph_for_connection(connection_kind));

        if column_index + 1 < lane_width {
            output.push(config.glyph_for_connection(ConnectionKind::Empty));
        }
    }

    true
}

fn render_fingerprint<T: GraphNode>(
    nodes: &[T],
    content_by_node: Option<&[Cow<'_, str>]>,
    _config: &RenderConfig,
) -> u64 {
    let mut hasher = DefaultHasher::new();

    nodes.len().hash(&mut hasher);
    for (node_index, node) in nodes.iter().enumerate() {
        node.id().hash(&mut hasher);
        node.parents().len().hash(&mut hasher);
        for parent_id in node.parents() {
            parent_id.hash(&mut hasher);
        }
        content_by_node
            .and_then(|content| content.get(node_index))
            .map_or("", |content| content.as_ref())
            .hash(&mut hasher);
    }

    hasher.finish()
}
