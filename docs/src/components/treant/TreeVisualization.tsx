import { useMemo } from 'react';
import { hierarchy, tree as d3Tree } from 'd3-hierarchy';
import styles from './TreeVisualization.module.css';

interface TreeNode {
  visits: number;
  avg_reward: number;
  proven?: string;
  children: Array<{
    mov: string;
    visits: number;
    avg_reward: number;
    prior?: number;
    child?: TreeNode;
  }>;
}

interface TreeVisualizationProps {
  tree: TreeNode;
  maxDepth?: number;
  width?: number;
  height?: number;
  highlightPath?: string[];
}

interface FlatNode {
  name: string;
  visits: number;
  avg_reward: number;
  proven?: string;
  move?: string;
  children?: FlatNode[];
}

function flatten(node: TreeNode, depth: number, maxDepth: number, move?: string): FlatNode {
  const flat: FlatNode = {
    name: move ?? 'root',
    visits: node.visits,
    avg_reward: node.avg_reward,
    proven: node.proven,
    move,
  };

  if (depth < maxDepth && node.children && node.children.length > 0) {
    flat.children = node.children
      .filter((c) => c.child && c.child.visits > 0)
      .map((c) => flatten(c.child!, depth + 1, maxDepth, c.mov));
  }

  return flat;
}

function nodeRadius(visits: number): number {
  const r = 6 * Math.log2(visits + 1);
  return Math.max(12, Math.min(30, r));
}

function nodeColor(proven?: string): string {
  if (proven === 'Win') return '#22c55e';
  if (proven === 'Loss') return '#ef4444';
  if (proven === 'Draw') return '#eab308';
  return '#94a3b8';
}

export default function TreeVisualization({
  tree,
  maxDepth = 4,
  width = 600,
  height = 400,
  highlightPath,
}: TreeVisualizationProps) {
  const layout = useMemo(() => {
    if (!tree || tree.visits === 0) return null;

    const flat = flatten(tree, 0, maxDepth);
    const root = hierarchy(flat);

    const margin = { top: 40, right: 40, bottom: 40, left: 40 };
    const innerWidth = width - margin.left - margin.right;
    const innerHeight = height - margin.top - margin.bottom;

    const treeLayout = d3Tree<FlatNode>().size([innerWidth, innerHeight]);
    treeLayout(root);

    const highlightSet = new Set(highlightPath ?? []);

    const nodes = root.descendants().map((d) => ({
      x: d.x! + margin.left,
      y: d.y! + margin.top,
      data: d.data,
      r: nodeRadius(d.data.visits),
      fill: nodeColor(d.data.proven),
      highlighted: d.data.move ? highlightSet.has(d.data.move) : highlightSet.size === 0,
    }));

    const links = root.links().map((link) => ({
      x1: link.source.x! + margin.left,
      y1: link.source.y! + margin.top,
      x2: link.target.x! + margin.left,
      y2: link.target.y! + margin.top,
      move: link.target.data.move,
      highlighted: link.target.data.move
        ? highlightSet.has(link.target.data.move)
        : false,
    }));

    return { nodes, links };
  }, [tree, maxDepth, width, height, highlightPath]);

  if (!layout) return null;

  return (
    <svg
      className={styles.tree}
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
    >
      {layout.links.map((link, i) => (
        <g key={`link-${i}`}>
          <line
            className={styles.edge}
            x1={link.x1}
            y1={link.y1}
            x2={link.x2}
            y2={link.y2}
            data-highlighted={link.highlighted || undefined}
          />
          {link.move && (
            <text
              className={styles.edgeLabel}
              x={(link.x1 + link.x2) / 2}
              y={(link.y1 + link.y2) / 2 - 4}
              textAnchor="middle"
            >
              {link.move}
            </text>
          )}
        </g>
      ))}
      {layout.nodes.map((node, i) => (
        <g key={`node-${i}`}>
          <circle
            className={styles.node}
            cx={node.x}
            cy={node.y}
            r={node.r}
            fill={node.fill}
            data-highlighted={node.highlighted || undefined}
          />
          <text
            className={styles.nodeLabel}
            x={node.x}
            y={node.y}
            textAnchor="middle"
            dominantBaseline="central"
          >
            {node.data.visits}
          </text>
        </g>
      ))}
    </svg>
  );
}
