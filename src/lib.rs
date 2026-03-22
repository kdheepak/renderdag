pub mod graph;

pub use graph::{
    ColumnComputation, ConnectionKind, GraphLayout, GraphNode, GraphRenderer, LaneId, MaskRoute, MissingParentState,
    Node, NodeId, NodeKind, ParentAvailability, RenderConfig, Renderable, RouteReason, RowPlan, StepDetails,
    TrackCell,
};

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;

    static COLOR_EYRE_INIT: Once = Once::new();

    #[must_use]
    pub fn parse_nodes(input: &str) -> color_eyre::eyre::Result<Vec<Node>> {
        fn parse_parent_list(part: &str) -> Vec<NodeId> {
            part.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
        }

        let mut nodes = Vec::new();

        for (idx, raw) in input.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw.trim();

            if line.is_empty() {
                continue;
            }

            let (id_part, parents_part) = match line.split_once(':') {
                Some((left, right)) => (left.trim(), Some(right)),
                None => (line, None),
            };

            if id_part.is_empty() {
                color_eyre::eyre::bail!("Line {}: Missing node ID", line_no);
            }

            let parents = parents_part.map(parse_parent_list).unwrap_or_default();
            nodes.push(Node::new(id_part, parents));
        }

        Ok(nodes)
    }

    fn diff_blocks(expected: &str, actual: &str) -> String {
        let mut msg = String::new();
        msg.push_str("\n\n--- expected ---\n\n");
        msg.push_str(expected);
        msg.push_str("\n\n--- actual ---\n\n");
        msg.push_str(actual);
        msg.push_str("\n");
        msg
    }

    pub fn test_output(data: &str, expected: &str) -> color_eyre::Result<()> {
        test_output_with_config(data, expected, RenderConfig::default())
    }

    pub fn test_output_with_config(data: &str, expected: &str, config: RenderConfig) -> color_eyre::Result<()> {
        use color_eyre::eyre::eyre;

        COLOR_EYRE_INIT.call_once(|| {
            let _ = color_eyre::install();
        });

        let nodes = parse_nodes(data).map_err(|e| eyre!("parse_nodes failed: {:?}", e))?;

        let mut r = GraphRenderer::new(config);
        let actual_glyphs = r.render_to_string(&nodes);

        let expected_n = expected.trim().to_string();

        // If the expected output contains node ids / parents, enrich the actual output the same way.
        let expected_has_suffix = expected_n.chars().any(|c| c.is_ascii_alphanumeric() || c == '(' || c == ')');

        let actual_pretty =
            if expected_has_suffix { render_with_suffix(&actual_glyphs, &nodes) } else { actual_glyphs };

        let actual_n = actual_pretty.trim().to_string();

        if expected_n != actual_n {
            println!("\n{}", diff_blocks(&expected_n, &actual_n));
        }
        pretty_assertions::assert_eq!(
            expected_n,
            actual_n,
            "Expected (left; `<`) output did not match Actual (right; `>`) output"
        );

        Ok(())
    }

    fn render_with_suffix(rendered: &str, nodes: &[Node]) -> String {
        let mut out = String::with_capacity(rendered.len() + nodes.len() * 16);

        let mut rendered_lines = rendered.lines();
        let mut first_line = true;

        for node in nodes {
            let Some(line) = rendered_lines.next() else { break };

            if !first_line {
                out.push('\n');
            }
            first_line = false;

            out.push_str(line);

            // " <id>"
            out.push(' ');
            out.push_str(&node.id);

            // " (<p1> <p2> ...)"
            if !node.parents.is_empty() {
                out.push(' ');
                out.push('(');
                for (pi, p) in node.parents.iter().enumerate() {
                    if pi != 0 {
                        out.push(' ');
                    }
                    out.push_str(p);
                }
                out.push(')');
            }
        }

        for line in rendered_lines {
            if !first_line {
                out.push('\n');
            }
            first_line = false;
            out.push_str(line);
        }

        out
    }

    #[test]
    fn test_linear_chain() -> color_eyre::Result<()> {
        test_output(
            r#"
1-d: 1-c
1-c: 1-b
1-b: 1-a
1-a
"#,
            r#"
⦿ 1-d (1-c)
● 1-c (1-b)
● 1-b (1-a)
⊝ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_with_single_side_commit() -> color_eyre::Result<()> {
        test_output(
            r#"
1-d: 1-c
1-c: 1-b, 2-a
2-a: 1-b
1-b: 1-a
1-a
"#,
            r#"
⦿ 1-d (1-c)
⊗─╮ 1-c (1-b 2-a)
│ ● 2-a (1-b)
●─╯ 1-b (1-a)
⊝ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_with_extra_mainline_commit() -> color_eyre::Result<()> {
        test_output(
            r#"
1-e: 1-d
1-d: 1-c, 2-a
2-a: 1-b
1-c: 1-b
1-b: 1-a
1-a
"#,
            r#"
⦿ 1-e (1-d)
⊗─╮ 1-d (1-c 2-a)
│ ● 2-a (1-b)
● │ 1-c (1-b)
●─╯ 1-b (1-a)
⊝ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_with_extra_commits_on_both_branches() -> color_eyre::Result<()> {
        test_output(
            r#"
1-e: 1-d
1-d: 1-c, 2-b
2-b: 2-a
2-a: 1-b
1-c: 1-b
1-b: 1-a
1-a
"#,
            r#"
⦿ 1-e (1-d)
⊗─╮ 1-d (1-c 2-b)
│ ● 2-b (2-a)
│ ● 2-a (1-b)
● │ 1-c (1-b)
●─╯ 1-b (1-a)
⊝ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_with_crossover_left_variant() -> color_eyre::Result<()> {
        test_output(
            r#"
f: e, c
e: b, d
d: a
c: a
b: a
a
"#,
            r#"
⍟─╮ f (e c)
⊗─┊─╮ e (b d)
│ │ ● d (a)
│ ● │ c (a)
● │ │ b (a)
⊝─┴─╯ a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_with_crossover_right_variant() -> color_eyre::Result<()> {
        test_output(
            r#"
f: e, d
e: b, c
d: a
c: a
b: a
a
"#,
            r#"
⍟─╮ f (e d)
⊗─┊─╮ e (b c)
│ ● │ d (a)
│ │ ● c (a)
● │ │ b (a)
⊝─┴─╯ a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_whose_parent_is_itself_a_merge() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C
C: B, D
B: D
D
"#,
            r#"
⍟─╮ A (B C)
│ ⊗─╮ C (B D)
●─╯ │ B (D)
⊝───╯ D
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_with_shared_ancestor_parent() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, D
B: D, C
C: D
D
"#,
            r#"
⍟─╮ A (B D)
⊗─┊─╮ B (D C)
│ │ ● C (D)
⊝─┴─╯ D
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_octopus_merge_with_many_parents() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E, F, G, H, I, J, K
B: K
C: K
D: K
E: K
F: K
G: K
H: K
I: K
J: K
K
"#,
            r#"
⍟─┬─┬─┬─┬─┬─┬─┬─┬─╮ A (B C D E F G H I J K)
● │ │ │ │ │ │ │ │ │ B (K)
│ ● │ │ │ │ │ │ │ │ C (K)
│ │ ● │ │ │ │ │ │ │ D (K)
│ │ │ ● │ │ │ │ │ │ E (K)
│ │ │ │ ● │ │ │ │ │ F (K)
│ │ │ │ │ ● │ │ │ │ G (K)
│ │ │ │ │ │ ● │ │ │ H (K)
│ │ │ │ │ │ │ ● │ │ I (K)
│ │ │ │ │ │ │ │ ● │ J (K)
⊝─┴─┴─┴─┴─┴─┴─┴─┴─╯ K
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_octopus_merge_with_nested_merge_parent() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E
C: E
E: D, B
B: D
D
"#,
            r#"
⍟─┬─┬─╮ A (B C D E)
│ ● │ │ C (E)
│ ⊗─┊─┴─╮ E (D B)
●─┊─┊───╯ B (D)
⊝─┴─╯ D
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_three_merges_with_nested_branch_merge() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C
B: D, E
C: F
F: G, D
D: G
G: E
E
"#,
            r#"
⍟─╮ A (B C)
⊗─┊─╮ B (D E)
│ ● │ C (F)
│ ⊗─┊─╮ F (G D)
●─┊─┊─╯ D (G)
●─╯ │ G (E)
⊝───╯ E
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_disconnected_histories_without_common_root() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C
C: D
B: E, D
D: F
E
F
"#,
            r#"
⍟─╮ A (B C)
│ ● C (D)
⊗─┊─╮ B (E D)
│ ●─╯ D (F)
⊝ │ E
  ⊝ F
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_disconnected_history_with_missing_parents() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C
C: B
D: B
E: F
F: G
G: H
"#,
            r#"
⊛─╮ A (B C)
│ ◌ C (B)
│ │ ◌ D (B)
│ │ │ ⦿ E (F)
│ │ │ ● F (G)
│ │ │ ◌ G (H)
"#,
        )?;
        Ok(())
    }

    #[test]
    fn collapses_lanes_in_linearized_multi_parent_chain() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E, F, G
B: C
C: D
D: E
E: F
F: G
G
"#,
            r#"
⍟─┬─┬─┬─┬─╮ A (B C D E F G)
● │ │ │ │ │ B (C)
●─╯ │ │ │ │ C (D)
●───╯ │ │ │ D (E)
●─────╯ │ │ E (F)
●───────╯ │ F (G)
⊝─────────╯ G
"#,
        )?;

        Ok(())
    }

    #[test]
    fn collapses_single_lane_after_merge_resolution() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E, F
C: B
B: H
F: E
H: D
D: E
E
"#,
            r#"
⍟─┬─┬─┬─╮ A (B C D E F)
│ ● │ │ │ C (B)
●─╯ │ │ │ B (H)
│ ╭─╯ │ ● F (E)
● │ ╭─╯ │ H (D)
●─╯ │ ╭─╯ D (E)
⊝───┴─╯ E
"#,
        )?;

        Ok(())
    }

    #[test]
    fn collapses_multiple_lanes_in_one_pass() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E, F
C: B
B: H
F: G
H: D
D: E
E: G
G
"#,
            r#"
⍟─┬─┬─┬─╮ A (B C D E F)
│ ● │ │ │ C (B)
●─╯ │ │ │ B (H)
│ ╭─╯ │ ● F (G)
● │ ╭─╯ │ H (D)
●─╯ │ ╭─╯ D (E)
●───╯ │ E (G)
⊝─────╯ G
"#,
        )?;

        Ok(())
    }

    #[test]
    fn collapses_multiple_lanes_across_several_rows() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, K, D, E, F
C: B
B: K
K: H
F: G
H: D
D: E
E: G
G
"#,
            r#"
⍟─┬─┬─┬─┬─╮ A (B C K D E F)
│ ● │ │ │ │ C (B)
●─╯ │ │ │ │ B (K)
●───╯ │ │ │ K (H)
│ ╭───╯ │ ● F (G)
● │ ╭───╯ │ H (D)
●─╯ │ ╭───╯ D (E)
●───╯ │ E (G)
⊝─────╯ G
"#,
        )?;

        Ok(())
    }

    #[test]
    fn collapses_all_possible_lanes_in_dense_graph() -> color_eyre::Result<()> {
        test_output(
            r#"
A: Z, B, C, D, E, F
C: B
B: H
F: G
H: D
D: E
E: G
G: Z
Z
"#,
            r#"
⍟─┬─┬─┬─┬─╮ A (Z B C D E F)
│ │ ● │ │ │ C (B)
│ ●─╯ │ │ │ B (H)
│ │ ╭─╯ │ ● F (G)
│ ● │ ╭─╯ │ H (D)
│ ●─╯ │ ╭─╯ D (E)
│ ●───╯ │ E (G)
│ ●─────╯ G (Z)
⊝─╯ Z
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_large_disconnected_history_with_many_missing_parents() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C
C: B
D: B
E: F
F: G
G: H
H: I
I: J
J: K
K: L
L: M
M: N
N: O
O: P
B: Q, R
R: Q
Q: S, T
T: S
P: U
U: V
V: W
S: X, Y
Z: 1
Y: X
X: 2, 3
3: 2
4: 5
5: 6
W: 7
6: 8
7: 9
2: 0, a
a: b
b: c
c: d, 0
8: e
e: f
f: g
0: h, i
g: j
j: k
k: l
l: m
m: n
n: o
o: p
p: 1
9: q
1: r
i: h
d: h
r: s
h: t, u
s: v
u: t
w: x
x: y
"#,
            r#"
⍟─╮ A (B C)
│ ● C (B)
│ │ ⦿ D (B)
│ │ │ ⦿ E (F)
│ │ │ ● F (G)
│ │ │ ● G (H)
│ │ │ ● H (I)
│ │ │ ● I (J)
│ │ │ ● J (K)
│ │ │ ● K (L)
│ │ │ ● L (M)
│ │ │ ● M (N)
│ │ │ ● N (O)
│ │ │ ● O (P)
⊗─┴─┴─┊─╮ B (Q R)
│ ╭───╯ ● R (Q)
⊗─┊─┬───╯ Q (S T)
│ │ ● T (S)
│ ● │ P (U)
│ ● │ U (V)
│ ● │ V (W)
⊗─┊─┴─╮ S (X Y)
│ │ ⦿ │ Z (1)
│ │ │ ● Y (X)
⊗─┊─┊─┴─╮ X (2 3)
│ │ │ ●─╯ 3 (2)
│ │ │ │ ⦿ 4 (5)
│ │ │ │ ● 5 (6)
│ ● │ │ │ W (7)
│ │ │ │ ● 6 (8)
│ ● │ │ │ 7 (9)
⊗─┊─┊─┴─┊─╮ 2 (0 a)
│ │ │ ╭─╯ ● a (b)
│ │ │ │ ●─╯ b (c)
│ │ │ │ ⊗─╮ c (d 0)
│ │ │ ● │ │ 8 (e)
│ │ │ ● │ │ e (f)
│ │ │ ● │ │ f (g)
⊗─┊─┊─┊─┊─┴─╮ 0 (h i)
│ │ │ ● │ ╭─╯ g (j)
│ │ │ ● │ │ j (k)
│ │ │ ● │ │ k (l)
│ │ │ ● │ │ l (m)
│ │ │ ● │ │ m (n)
│ │ │ ● │ │ n (o)
│ │ │ ● │ │ o (p)
│ │ │ ● │ │ p (1)
│ ◌ │ │ │ │ 9 (q)
│ │ ●─╯ │ │ 1 (r)
│ │ │ ╭─╯ ● i (h)
│ │ │ ● ╭─╯ d (h)
│ │ ● │ │ r (s)
⊘─┊─┊─┴─┴─╮ h (t u)
│ │ ◌ ╭───╯ s (v)
│ │ │ ◌ u (t)
│ │ │ │ ⦿ w (x)
│ │ │ │ ◌ x (y)
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_parallel_sibling_merges() -> color_eyre::Result<()> {
        test_output(
            r#"
A: Z, B, C
B: D, C
D: E, C
E: Z
Z: Y
Y: X
X: W
W: C
C
"#,
            r#"
⍟─┬─╮ A (Z B C)
│ ⊗─┊─╮ B (D C)
│ ⊗─┊─┊─╮ D (E C)
│ ● │ │ │ E (Z)
●─╯ │ │ │ Z (Y)
● ╭─╯ │ │ Y (X)
● │ ╭─╯ │ X (W)
● │ │ ╭─╯ W (C)
⊝─┴─┴─╯ C
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_merge_path_shorter_than_parallel_branch() -> color_eyre::Result<()> {
        test_output(
            r#"
A: H, B, C, D, J, E
B: K
C: H
J: I
D: K
E: F
K: F
F: G, H
G: I
H: I
I
"#,
            r#"
⍟─┬─┬─┬─┬─╮ A (H B C D J E)
│ ● │ │ │ │ B (K)
│ │ ● │ │ │ C (H)
│ │ │ │ ● │ J (I)
│ │ │ ● │ │ D (K)
│ │ │ │ │ ● E (F)
│ ●─┊─╯ │ │ K (F)
│ ⊗─┊─┬─┊─╯ F (G H)
│ ● │ │ │ G (I)
●─┊─┴─╯ │ H (I)
⊝─┴─────╯ I
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_branch_and_merge_with_middle_divergence() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, E
B: F
C: F
E: Z
F: G, I
G: H, I
H: I
I: Z
Z
"#,
            r#"
⍟─┬─╮ A (B C E)
● │ │ B (F)
│ ● │ C (F)
│ │ ● E (Z)
⊗─┴─┊─╮ F (G I)
⊗─╮ │ │ G (H I)
● │ │ │ H (I)
●─┴─┊─╯ I (Z)
⊝───╯ Z
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_crossover_then_lane_collapse() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, D, E
B: F
D: F
E: Z
F: G
G: H, I
H: I
I: Z
Z
"#,
            r#"
⍟─┬─╮ A (B D E)
● │ │ B (F)
│ ● │ D (F)
│ │ ● E (Z)
●─╯ │ F (G)
⊗─╮ │ G (H I)
● │ │ H (I)
●─╯ │ I (Z)
⊝───╯ Z
"#,
        )?;

        Ok(())
    }

    #[test]
    fn collapses_parallel_branches_with_distinct_middle_paths() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E, F
D: I
E: J
F: I
B: C
C: G
G: H
H: I
I: J
J
"#,
            r#"
⍟─┬─┬─┬─╮ A (B C D E F)
│ │ ● │ │ D (I)
│ │ │ ● │ E (J)
│ │ │ │ ● F (I)
● │ │ │ │ B (C)
●─╯ │ │ │ C (G)
● ╭─╯ │ │ G (H)
● │ ╭─╯ │ H (I)
●─┴─┊───╯ I (J)
⊝───╯ J
"#,
        )?;

        Ok(())
    }

    #[test]
    fn collapses_long_multi_stage_lane_chain() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E, F, G
C: B
D: B
E: B
F: B
B: H
H: I, G
I: J
G: J
J: K
K: L, M, N, O, P, Q
M: L
N: L
O: L
P: L
L: R
R: S, T
S: Q
T: Q
Q
"#,
            r#"
⍟─┬─┬─┬─┬─╮ A (B C D E F G)
│ ● │ │ │ │ C (B)
│ │ ● │ │ │ D (B)
│ │ │ ● │ │ E (B)
│ │ │ │ ● │ F (B)
●─┴─┴─┴─╯ │ B (H)
⊗─╮ ╭─────╯ H (I G)
● │ │ I (J)
│ ●─╯ G (J)
●─╯ J (K)
⊗─┬─┬─┬─┬─╮ K (L M N O P Q)
│ ● │ │ │ │ M (L)
│ │ ● │ │ │ N (L)
│ │ │ ● │ │ O (L)
│ │ │ │ ● │ P (L)
●─┴─┴─┴─╯ │ L (R)
⊗─╮ ╭─────╯ R (S T)
● │ │ S (Q)
│ ● │ T (Q)
⊝─┴─╯ Q
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_deep_history_with_many_merges() -> color_eyre::Result<()> {
        test_output(
            r#"
P: N, O
O: M
N: L, M
M: K
L: J, K
K: E, J
J: I, F
I: C, H
H: G
F: C
E: D
C: A, B
D: Q
B: R
R: A
Q: X
A: S, T
T: U
U: V
V: W
"#,
            r#"
⍟─╮ P (N O)
│ ● O (M)
⊗─┊─╮ N (L M)
│ ●─╯ M (K)
⊗─┊─╮ L (J K)
│ ⊗─┴─╮ K (E J)
⊗─┊─┬─╯ J (I F)
⊗─┊─┊─╮ I (C H)
│ │ │ ◌ H (G)
│ │ ● │ F (C)
│ ● │ │ E (D)
⊗─┊─┴─┊─╮ C (A B)
│ ● ╭─╯ │ D (Q)
│ │ │ ●─╯ B (R)
│ │ │ ● R (A)
│ ◌ │ │ Q (X)
⊘─┊─┊─┴─╮ A (S T)
│ │ │ ●─╯ T (U)
│ │ │ ● U (V)
│ │ │ ◌ V (W)
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_highly_tangled_merge_graph() -> color_eyre::Result<()> {
        test_output(
            r#"
1-l: 1-k, 2-d
1-k: 1-j, 3-b, 4-a
2-d: 1-i
1-j: 1-i
3-b: 1-i
4-a: 1-i
1-i: 1-h, 2-c
1-h: 1-g, 5-a
2-c: 2-b, 3-a
1-g: 1-f
5-a: 3-a
2-b: 2-a, 1-e
1-f: 1-e
3-a: 1-c
2-a: 1-b
1-e: 1-d
1-d: 1-c
1-c: 1-b
1-b: 1-a
1-a
"#,
            r#"
⍟─╮ 1-l (1-k 2-d)
⊗─┊─┬─╮ 1-k (1-j 3-b 4-a)
│ ● │ │ 2-d (1-i)
● │ │ │ 1-j (1-i)
│ │ ● │ 3-b (1-i)
│ │ │ ● 4-a (1-i)
⊗─┴─┴─┴─╮ 1-i (1-h 2-c)
⊗─╮ ╭───╯ 1-h (1-g 5-a)
│ │ ⊗─╮ 2-c (2-b 3-a)
● │ │ │ 1-g (1-f)
│ ● │ │ 5-a (3-a)
│ │ ⊗─┊─╮ 2-b (2-a 1-e)
● │ │ │ │ 1-f (1-e)
│ ●─┊─╯ │ 3-a (1-c)
│ │ ● ╭─╯ 2-a (1-b)
●─┊─┊─╯ 1-e (1-d)
● │ │ 1-d (1-c)
●─╯ │ 1-c (1-b)
●───╯ 1-b (1-a)
⊝ 1-a
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_four_parent_octopus_merge() -> color_eyre::Result<()> {
        test_output(
            r#"
1-c: 1-b, 2-a, 3-a, 4-a
4-a: 1-a
3-a: 1-a
2-a: 1-a
1-b: 1-a
1-a
"#,
            r#"
⍟─┬─┬─╮ 1-c (1-b 2-a 3-a 4-a)
│ │ │ ● 4-a (1-a)
│ │ ● │ 3-a (1-a)
│ ● │ │ 2-a (1-a)
● │ │ │ 1-b (1-a)
⊝─┴─┴─╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_branch_end_with_multi_parent_rejoin() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-e: 1-c, 2-b, 3-b, 4-b
    4-b: 4-a
    3-b: 3-a
    2-b: 2-a
    2-a: 1-c, 3-a, 4-a
    4-a: 1-a
    3-a: 1-a
    1-c: 1-b
    1-b: 1-a
    1-a
"#,
            r#"
⍟─┬─┬─╮ 1-e (1-c 2-b 3-b 4-b)
│ │ │ ● 4-b (4-a)
│ │ ● │ 3-b (3-a)
│ ● │ │ 2-b (2-a)
│ ⊗─┊─┊─┬─╮ 2-a (1-c 3-a 4-a)
│ │ │ ●─┊─╯ 4-a (1-a)
│ │ ●─┊─╯ 3-a (1-a)
●─╯ │ │ 1-c (1-b)
● ╭─╯ │ 1-b (1-a)
⊝─┴───╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_three_node_linear_history() -> color_eyre::Result<()> {
        test_output(
            r#"
a: b
b: c
c
"#,
            r#"
⦿ a (b)
● b (c)
⊝ c
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_basic_branch_and_merge() -> color_eyre::Result<()> {
        test_output(
            r#"
d: b, c
c: a
b: a
a
"#,
            r#"
⍟─╮ d (b c)
│ ● c (a)
● │ b (a)
⊝─╯ a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_large_multi_merge() -> color_eyre::Result<()> {
        test_output(
            r#"
A: B, C, D, E
D: B
C: F
E: H
B: H, F
F: G
G: H
H: I
I: J, K, L, M, P, O
K: J
J: N
N: O
O: Q
Q: R, S
R: T, U
T: V, W
V: X, W
X: Y, Z
W
QQ: M
M: Y, A1, A2
P: Y
Z: Y
Y: A1
A1: A2
A2: S
S: U
U: A3
A3: A4, A5, A8
A4: A6, A5
A5: A7, A9
A6: AA, A9
AA
L
A7
A8
A9
"#,
            r#"
⍟─┬─┬─╮ A (B C D E)
│ │ ● │ D (B)
│ ● │ │ C (F)
│ │ │ ● E (H)
⊗─┊─┴─┊─╮ B (H F)
│ ●───┊─╯ F (G)
│ ● ╭─╯ G (H)
●─┴─╯ H (I)
⊗─┬─┬─┬─┬─╮ I (J K L M P O)
│ ● │ │ │ │ K (J)
●─╯ │ │ │ │ J (N)
● ╭─╯ │ │ │ N (O)
●─┊───┊─┊─╯ O (Q)
⊗─┊─╮ │ │ Q (R S)
⊗─┊─┊─┊─┊─╮ R (T U)
⊗─┊─┊─┊─┊─┊─╮ T (V W)
⊗─┊─┊─┊─┊─┊─┊─╮ V (X W)
⊗─┊─┊─┊─┊─┊─┊─┊─╮ X (Y Z)
│ │ │ │ │ │ ⊝─╯ │ W
│ │ │ │ │ │ ⦿ ╭─╯ QQ (M)
│ │ │ ⊗─┊─┊─┴─┊─┬─╮ M (Y A1 A2)
│ │ │ │ ● │ ╭─╯ │ │ P (Y)
│ │ │ │ │ │ ● ╭─╯ │ Z (Y)
●─┊─┊─┴─┴─┊─╯ │ ╭─╯ Y (A1)
●─┊─┊─────┊───╯ │ A1 (A2)
●─┊─┊─────┊─────╯ A2 (S)
●─┊─╯ ╭───╯ S (U)
●─┊───╯ U (A3)
⊗─┊─┬─╮ A3 (A4 A5 A8)
⊗─┊─┊─┊─╮ A4 (A6 A5)
│ │ ⊗─┊─┴─╮ A5 (A7 A9)
⊗─┊─┊─┊─╮ │ A6 (AA A9)
⊝ │ │ │ │ │ AA
  ⊝ │ │ │ │ L
    ⊝ │ │ │ A7
      ⊝ │ │ A8
        ⊝─╯ A9
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_orphan_isolated_node() -> color_eyre::Result<()> {
        test_output(
            r#"
i: h
h: g, d
g: f, e
f: c
e: c
d: c
c: b
b: a
"#,
            r#"
⦿ i (h)
⊗─╮ h (g d)
⊗─┊─╮ g (f e)
● │ │ f (c)
│ │ ● e (c)
│ ● │ d (c)
●─┴─╯ c (b)
◌ b (a)
"#,
        )?;

        Ok(())
    }

    #[test]
    fn partially_missing_parents_are_exposed_in_the_model() -> color_eyre::Result<()> {
        let nodes = parse_nodes(
            r#"
Y: X
X: A, Z
A
"#,
        )?;

        let mut layout = GraphLayout::new();
        let plans = layout.layout(&nodes);

        assert_eq!(plans[0].parent_availability, ParentAvailability::new(1, 0));
        assert_eq!(plans[0].parent_availability.missing_parent_state(), MissingParentState::None);

        assert_eq!(plans[1].parent_availability, ParentAvailability::new(1, 1));
        assert_eq!(plans[1].parent_availability.missing_parent_state(), MissingParentState::Some);

        assert_eq!(plans[2].parent_availability, ParentAvailability::new(0, 0));
        assert_eq!(plans[2].parent_availability.missing_parent_state(), MissingParentState::None);

        Ok(())
    }

    #[test]
    fn partial_parent_omissions_render_as_truncated_merges() -> color_eyre::Result<()> {
        test_output(
            r#"
Y: X
X: A, Z
A
"#,
            r#"
⦿ Y (X)
⊘─╮ X (A Z)
⊝ │ A
"#,
        )?;

        test_output(
            r#"
X: A, Z
A
"#,
            r#"
⊛─╮ X (A Z)
⊝ │ A
"#,
        )?;

        Ok(())
    }

    #[test]
    fn terminal_lane_caps_can_be_enabled() -> color_eyre::Result<()> {
        let mut config = RenderConfig::default();
        config.set_render_terminal_lanes(true);

        test_output_with_config(
            r#"
Y: X
X: A, Z
A
"#,
            r#"
⦿ Y (X)
⊘─╮ X (A Z)
⊝ │ A
  ╵
"#,
            config,
        )?;

        Ok(())
    }

    #[test]
    fn test_cross_merge() -> color_eyre::Result<()> {
        test_output(
            r#"
1-e: 1-d, 2-c, 3-b
3-b: 3-a
2-c: 2-b
2-b: 2-a, 1-c, 3-a
3-a: 1-a
2-a: 1-b
1-d: 1-c
1-c: 1-b
1-b: 1-a
1-a
"#,
            r#"
⍟─┬─╮ 1-e (1-d 2-c 3-b)
│ │ ● 3-b (3-a)
│ ● │ 2-c (2-b)
│ ⊗─┊─┬─╮ 2-b (2-a 1-c 3-a)
│ │ ●─┊─╯ 3-a (1-a)
│ ● │ │ 2-a (1-b)
● │ │ │ 1-d (1-c)
●─┊─┊─╯ 1-c (1-b)
●─╯ │ 1-b (1-a)
⊝───╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_cross_merge_fan_out() -> color_eyre::Result<()> {
        test_output(
            r#"
1-e: 1-d, 2-c, 3-b
1-d: 1-c
2-c: 2-b
3-b: 3-a
2-b: 2-a, 1-c, 3-a
1-c: 1-b
2-a: 1-b
3-a: 1-a
1-b: 1-a
1-a
"#,
            r#"
⍟─┬─╮ 1-e (1-d 2-c 3-b)
● │ │ 1-d (1-c)
│ ● │ 2-c (2-b)
│ │ ● 3-b (3-a)
│ ⊗─┊─┬─╮ 2-b (2-a 1-c 3-a)
●─┊─┊─╯ │ 1-c (1-b)
│ ● │ ╭─╯ 2-a (1-b)
│ │ ●─╯ 3-a (1-a)
●─╯ │ 1-b (1-a)
⊝───╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_cross_merge_with_extra_crossover() -> color_eyre::Result<()> {
        test_output(
            r#"
1-e: 1-d, 2-c, 3-b
3-b: 3-a, 2-b
2-c: 2-b
2-b: 2-a, 1-c, 3-a
3-a: 1-a
2-a: 1-b
1-d: 1-c
1-c: 1-b
1-b: 1-a
1-a
"#,
            r#"
⍟─┬─╮ 1-e (1-d 2-c 3-b)
│ │ ⊗─╮ 3-b (3-a 2-b)
│ ● │ │ 2-c (2-b)
│ ⊗─┊─┴─┬─╮ 2-b (2-a 1-c 3-a)
│ │ ●───┊─╯ 3-a (1-a)
│ ● │ ╭─╯ 2-a (1-b)
● │ │ │ 1-d (1-c)
●─┊─┊─╯ 1-c (1-b)
●─╯ │ 1-b (1-a)
⊝───╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_complex_cross_merge_with_multiple_crossovers() -> color_eyre::Result<()> {
        test_output(
            r#"
1-e: 1-d, 2-c, 3-b, 1-b
1-d: 1-c
3-b: 3-a, 2-b, 1-b, 2-c
2-c: 2-b
2-b: 2-a, 1-c, 3-a, 1-b
1-c: 1-b
2-a: 1-b
3-a: 1-a
1-b
1-a
"#,
            r#"
⍟─┬─┬─╮ 1-e (1-d 2-c 3-b 1-b)
● │ │ │ 1-d (1-c)
│ │ ⊗─┊─┬─┬─╮ 3-b (3-a 2-b 1-b 2-c)
│ ●─┊─┊─┊─┊─╯ 2-c (2-b)
│ ⊗─┊─┊─┴─┊─┬─┬─╮ 2-b (2-a 1-c 3-a 1-b)
●─┊─┊─┊───┊─╯ │ │ 1-c (1-b)
│ ● │ │ ╭─╯ ╭─╯ │ 2-a (1-b)
│ │ ●─┊─┊───╯ ╭─╯ 3-a (1-a)
⊝─┴─┊─┴─┴─────╯ 1-b
    ⊝ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_dense_interwoven_merge_fixture_with_hook_avoiding_base_vertical() -> color_eyre::Result<()> {
        test_output(
            r#"
G: A, D, B, E, F
F: C, A, D, E, B
E: D, A
D: B, C
C: B, A
B: A
A
"#,
            r#"
⍟─┬─┬─┬─╮ G (A D B E F)
│ │ │ │ ⊗─┬─┬─┬─╮ F (C A D E B)
│ │ │ ⊗─┊─┊─┊─┴─┊─╮ E (D A)
│ ⊗─┊─┴─┊─┊─┴─╮ │ │ D (B C)
│ │ │ ⊗─┴─┊─┬─╯ │ │ C (B A)
│ ●─┴─┴───┊─┊───╯ │ B (A)
⊝─┴───────┴─┴─────╯ A
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_dense_interwoven_merge_fixture_with_early_collapse_hook() -> color_eyre::Result<()> {
        test_output(
            r#"
    G: B, D, E, C, F, A
    F: B, E, C, A, D
    E: B, A, D
    D: C
    C: B, A
    B: A
    A
"#,
            r#"
⍟─┬─┬─┬─┬─╮ G (B D E C F A)
│ │ │ │ ⊗─┊─┬─┬─┬─╮ F (B E C A D)
│ │ ⊗─┊─┊─┊─┴─┊─┊─┊─┬─╮ E (B A D)
│ ●─┊─┊─┊─┊───┊─┊─┴─┊─╯ D (C)
│ ⊗─┊─┴─┊─┊─┬─╯ │ ╭─╯ C (B A)
●─┴─┴───╯ │ │ ╭─╯ │ B (A)
⊝─────────┴─┴─┴───╯ A
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_multiple_disconnected_families() -> color_eyre::Result<()> {
        test_output(
            r#"
    H: F
    G: D
    F: E
    E: B
    D: C
    C
    B: A
    A
"#,
            r#"
⦿ H (F)
│ ⦿ G (D)
● │ F (E)
● │ E (B)
│ ● D (C)
│ ⊝ C
● B (A)
⊝ A
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_stacked_merges_fixture_with_early_collapse() -> color_eyre::Result<()> {
        test_output(
            r#"
1-d: 1-c
2-a: 1-c
3-a: 1-a
1-c: 1-b, 1-a
1-b: 1-a
1-a
"#,
            r#"
⦿ 1-d (1-c)
│ ⦿ 2-a (1-c)
│ │ ⦿ 3-a (1-a)
⊗─┴─┊─╮ 1-c (1-b 1-a)
● ╭─╯ │ 1-b (1-a)
⊝─┴───╯ 1-a
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_stacked_merges_fixture_cascade_out_with_all_crossover() -> color_eyre::Result<()> {
        test_output(
            r#"
1-d: 1-c, 2-a
2-a: 1-c
3-a: 1-b, 4-a
4-a: 1-b, 5-a
5-a: 1-b, 1-a
1-c: 1-b, 1-a
1-b: 1-a
1-a
"#,
            r#"
⍟─╮ 1-d (1-c 2-a)
│ ● 2-a (1-c)
│ │ ⍟─╮ 3-a (1-b 4-a)
│ │ │ ⊗─╮ 4-a (1-b 5-a)
│ │ │ │ ⊗─╮ 5-a (1-b 1-a)
⊗─┴─┊─┊─┊─┊─╮ 1-c (1-b 1-a)
●───┴─┴─╯ │ │ 1-b (1-a)
⊝─────────┴─╯ 1-a
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_stacked_merges_fixture_cascade_out_with_one_crossover() -> color_eyre::Result<()> {
        test_output(
            r#"
1-d: 1-c, 2-a
2-a: 1-c
1-c: 1-b, 1-a
3-a: 1-b, 4-a
4-a: 1-b, 5-a
5-a: 1-b, 1-a
1-b: 1-a
1-a
"#,
            r#"
⍟─╮ 1-d (1-c 2-a)
│ ● 2-a (1-c)
⊗─┴─╮ 1-c (1-b 1-a)
│ ⍟─┊─╮ 3-a (1-b 4-a)
│ │ │ ⊗─╮ 4-a (1-b 5-a)
│ │ │ │ ⊗─╮ 5-a (1-b 1-a)
●─┴─┊─┴─╯ │ 1-b (1-a)
⊝───┴─────╯ 1-a
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_high_fan_in_merge_fixture() -> color_eyre::Result<()> {
        test_output(
            r#"
1-c: 1-b, 2-a, 3-b, 4-a, 1-a, 3-a
2-a: 4-a, 3-a, 1-b, 1-a
4-a: 1-a, 3-a, 1-b
3-b: 1-a, 3-a
3-a: 1-b
1-b: 1-a
1-a
"#,
            r#"
⍟─┬─┬─┬─┬─╮ 1-c (1-b 2-a 3-b 4-a 1-a 3-a)
│ ⊗─┊─┊─┊─┊─┬─┬─╮ 2-a (4-a 3-a 1-b 1-a)
│ ⊗─┊─┴─┊─┊─┊─┊─┊─┬─╮ 4-a (1-a 3-a 1-b)
│ │ ⊗─╮ │ │ │ │ │ │ │ 3-b (1-a 3-a)
│ │ │ ●─┊─┴─┴─┊─┊─╯ │ 3-a (1-b)
●─┊─┊─┴─┊─────┴─┊───╯ 1-b (1-a)
⊝─┴─┴───┴───────╯ 1-a
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_merge_with_reintroduced_mainline_parent_collapse_root() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-e: 1-d, 1-a, 2-a, 3-a, 1-c
    2-a: 3-a, 1-d, 1-a
    1-d: 1-c
    3-a: 1-a, 1-c
    1-c: 1-b
    1-b: 1-a
    1-a
"#,
            r#"
⍟─┬─┬─┬─╮ 1-e (1-d 1-a 2-a 3-a 1-c)
│ │ ⊗─┊─┊─┬─╮ 2-a (3-a 1-d 1-a)
●─┊─┊─┊─┊─╯ │ 1-d (1-c)
│ │ ⊗─┴─┊─╮ │ 3-a (1-a 1-c)
●─┊─┊───┴─╯ │ 1-c (1-b)
● │ │ ╭─────╯ 1-b (1-a)
⊝─┴─┴─╯ 1-a
"#,
        )?;
        Ok(())
    }

    #[test]
    fn test_descending_merge_stack_cascade_collapse() -> color_eyre::Result<()> {
        test_output(
            r#"
        1b: 1a, 2a, 3b, 3a, 4a, 5a
        4a: 3a, 5a, 3b, 2a
        5a: 1a, 3b, 2a
        3b: 1a
        3a: 2c
        2c: 2b
        2b: 2a
        2a: 1a
        1a
"#,
            r#"
⍟─┬─┬─┬─┬─╮ 1b (1a 2a 3b 3a 4a 5a)
│ │ │ │ ⊗─┊─┬─┬─╮ 4a (3a 5a 3b 2a)
│ │ │ │ │ ⊗─┴─┊─┊─┬─╮ 5a (1a 3b 2a)
│ │ ●─┊─┊─┊───┴─┊─╯ │ 3b (1a)
│ │ │ ●─╯ │ ╭───╯ ╭─╯ 3a (2c)
│ │ │ ● ╭─╯ │ ╭───╯ 2c (2b)
│ │ │ ● │ ╭─╯ │ 2b (2a)
│ ●─┊─┴─┊─┴───╯ 2a (1a)
⊝─┴─┴───╯ 1a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_multi_branch_rejoin_with_nested_side_branches() -> color_eyre::Result<()> {
        test_output(
            r#"
        1-d: 1-c, 2-a, 3-a, 4-b, 5-b
        5-b: 5-a
        5-a: 4-a
        4-b: 4-a
        4-a: 1-a, 1-c
        1-c: 1-b
        3-a: 1-b
        1-b: 1-a
        2-a: 1-a
        1-a
"#,
            r#"
⍟─┬─┬─┬─╮ 1-d (1-c 2-a 3-a 4-b 5-b)
│ │ │ │ ● 5-b (5-a)
│ │ │ │ ● 5-a (4-a)
│ │ │ ● │ 4-b (4-a)
│ │ │ ⊗─┴─╮ 4-a (1-a 1-c)
●─┊─┊─┊───╯ 1-c (1-b)
│ │ ● │ 3-a (1-b)
●─┊─╯ │ 1-b (1-a)
│ ● ╭─╯ 2-a (1-a)
⊝─┴─╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_three_way_merge_with_vertical_crossover() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-d: 1-c, 2-b, 3-b
    3-b: 3-a
    2-b: 2-a
    1-c: 1-b, 2-a, 3-a
    2-a: 1-b
    3-a: 1-a
    1-b: 1-a
    1-a
"#,
            r#"
⍟─┬─╮ 1-d (1-c 2-b 3-b)
│ │ ● 3-b (3-a)
│ ● │ 2-b (2-a)
⊗─┊─┊─┬─╮ 1-c (1-b 2-a 3-a)
│ ●─┊─╯ │ 2-a (1-b)
│ │ ●───╯ 3-a (1-a)
●─╯ │ 1-b (1-a)
⊝───╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_left_leaning_octopus_merge() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-c: 1-b
    2-b: 1-b, 4-a, 5-a, 2-a
    2-a: 1-a
    5-a: 1-a
    4-a: 1-a
    1-b: 1-a
    1-a
"#,
            r#"
⦿ 1-c (1-b)
│ ⍟─┬─┬─╮ 2-b (1-b 4-a 5-a 2-a)
│ │ │ │ ● 2-a (1-a)
│ │ │ ● │ 5-a (1-a)
│ │ ● │ │ 4-a (1-a)
●─╯ │ │ │ 1-b (1-a)
⊝───┴─┴─╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_octopus_merge_with_crossover() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-b: 1-a
    2-b: 2-a, 3-a, 4-a, 5-a
    5-a: 1-a
    4-a: 1-a
    3-a: 1-a
    2-a: 1-a
    1-a
"#,
            r#"
⦿ 1-b (1-a)
│ ⍟─┬─┬─╮ 2-b (2-a 3-a 4-a 5-a)
│ │ │ │ ● 5-a (1-a)
│ │ │ ● │ 4-a (1-a)
│ │ ● │ │ 3-a (1-a)
│ ● │ │ │ 2-a (1-a)
⊝─┴─┴─┴─╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_left_leaning_octopus_merge_with_crossover() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-c: 1-b
    2-b: 2-a, 3-a, 4-a, 1-b
    1-b: 1-a
    4-a: 1-a
    3-a: 1-a
    2-a: 1-a
    1-a
"#,
            r#"
⦿ 1-c (1-b)
│ ⍟─┬─┬─╮ 2-b (2-a 3-a 4-a 1-b)
●─┊─┊─┊─╯ 1-b (1-a)
│ │ │ ● 4-a (1-a)
│ │ ● │ 3-a (1-a)
│ ● │ │ 2-a (1-a)
⊝─┴─┴─╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_long_history_with_repeated_branch_merge_cycles() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-M: 1-L
    1-L: 1-K, 2-bb
    2-bb: 2-aa, 3-aa
    3-aa
    2-aa
    1-K: 1-J
    1-J: 1-I, 2-E
    2-E: 2-D, 3-B
    3-B: 3-A
    3-A: 2-B
    2-D: 2-C
    2-C: 1-F
    1-I: 1-H
    1-H: 1-G
    1-G: 1-F
    1-F: 1-E
    1-E: 1-D, 2-B
    2-B: 2-A
    2-A: 1-B
    1-D: 1-C
    1-C: 1-B
    1-B: 1-A
    1-A
"#,
            r#"
⦿ 1-M (1-L)
⊗─╮ 1-L (1-K 2-bb)
│ ⊗─╮ 2-bb (2-aa 3-aa)
│ │ ⊝ 3-aa
│ ⊝ 2-aa
● 1-K (1-J)
⊗─╮ 1-J (1-I 2-E)
│ ⊗─╮ 2-E (2-D 3-B)
│ │ ● 3-B (3-A)
│ │ ● 3-A (2-B)
│ ● │ 2-D (2-C)
│ ● │ 2-C (1-F)
● │ │ 1-I (1-H)
● │ │ 1-H (1-G)
● │ │ 1-G (1-F)
●─╯ │ 1-F (1-E)
⊗─╮ │ 1-E (1-D 2-B)
│ ●─╯ 2-B (2-A)
│ ● 2-A (1-B)
● │ 1-D (1-C)
● │ 1-C (1-B)
●─╯ 1-B (1-A)
⊝ 1-A
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_octopus_branches_followed_by_merge_with_hook_collapse() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-d: 1-c, 2-b, 3-b
    3-b: 1-c, 2-a, 3-a, 4-a, 5-a
    2-b: 1-c, 2-a, 3-a, 4-a, 5-a
    5-a: 1-b
    4-a: 1-b
    3-a: 1-a, 1-b
    2-a: 1-a, 1-b
    1-c: 1-b
    1-b: 1-aa
    1-aa: 1-a
    1-a
"#,
            r#"
⍟─┬─╮ 1-d (1-c 2-b 3-b)
│ │ ⊗─┬─┬─┬─╮ 3-b (1-c 2-a 3-a 4-a 5-a)
│ ⊗─┊─┊─┊─┊─┊─┬─┬─┬─╮ 2-b (1-c 2-a 3-a 4-a 5-a)
│ │ │ │ │ │ ●─┊─┊─┊─╯ 5-a (1-b)
│ │ │ │ │ ●─┊─┊─┊─╯ 4-a (1-b)
│ │ │ │ ⊗─┊─┊─┊─┴─╮ 3-a (1-a 1-b)
│ │ │ ⊗─┊─┊─┊─┴─╮ │ 2-a (1-a 1-b)
●─┴─╯ │ │ │ │ ╭─╯ │ 1-c (1-b)
●─────┊─┊─┴─┴─┴───╯ 1-b (1-aa)
● ╭───╯ │ 1-aa (1-a)
⊝─┴─────╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_branch_and_merge_near_root() -> color_eyre::Result<()> {
        test_output(
            r#"
    1-c: 1-b
    2-c: 1-b
    2-b: 2-a
    2-a: 1-a, 3-a
    3-a: 1-a
    1-b: 1-a
    1-a
"#,
            r#"
⦿ 1-c (1-b)
│ ⦿ 2-c (1-b)
│ │ ⦿ 2-b (2-a)
│ │ ⊗─╮ 2-a (1-a 3-a)
│ │ │ ● 3-a (1-a)
●─╯ │ │ 1-b (1-a)
⊝───┴─╯ 1-a
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_immediate_merge_after_parallel_branches() -> color_eyre::Result<()> {
        test_output(
            r#"
    F: C
    E: B
    D: A
    C: A, B
    B: A
    A
"#,
            r#"
⦿ F (C)
│ ⦿ E (B)
│ │ ⦿ D (A)
⊗─┊─┊─╮ C (A B)
│ ●─┊─╯ B (A)
⊝─┴─╯ A
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_branching_out_from_single_base() -> color_eyre::Result<()> {
        test_output(
            r#"
    E: A, B
    D: B
    C: B
    B: A
    A
"#,
            r#"
⍟─╮ E (A B)
│ │ ⦿ D (B)
│ │ │ ⦿ C (B)
│ ●─┴─╯ B (A)
⊝─╯ A
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_multiple_branches_merge_in_to_single_node() -> color_eyre::Result<()> {
        test_output(
            r#"
    F: B
    E: B, C, D
    D: A
    C: A
    B: A
    A
"#,
            r#"
⦿ F (B)
│ ⍟─┬─╮ E (B C D)
│ │ │ ● D (A)
│ │ ● │ C (A)
●─╯ │ │ B (A)
⊝───┴─╯ A
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_multiple_source_nodes_branching_in() -> color_eyre::Result<()> {
        test_output(
            r#"
    H: E
    G: E
    F: E, D, C
    E: B
    D: A
    C: A
    B: A
    A
"#,
            r#"
⦿ H (E)
│ ⦿ G (E)
│ │ ⍟─┬─╮ F (E D C)
●─┴─╯ │ │ E (B)
│ ●───╯ │ D (A)
│ │ ●───╯ C (A)
● │ │ B (A)
⊝─┴─╯ A
"#,
        )?;

        Ok(())
    }

    #[test]
    fn lane_collapse_shifts_lanes_left_when_possible_but_terminal_nodes_dont() -> color_eyre::Result<()> {
        test_output(
            r#"
X: A, B, C
A
B
C: D
D
"#,
            r#"
⍟─┬─╮ X (A B C)
⊝ │ │ A
  ⊝ │ B
●───╯ C (D)
⊝ D
"#,
        )?;

        Ok(())
    }

    #[test]
    fn orphan_nodes_are_marked_as_such_and_force_extra_lanes() -> color_eyre::Result<()> {
        test_output(
            r#"
X: Z
T
"#,
            r#"
◌ X (Z)
│ ⊝ T
"#,
        )?;

        Ok(())
    }
}
