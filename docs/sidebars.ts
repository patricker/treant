import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Tutorials',
      items: [
        'tutorials/01-what-is-mcts',
        'tutorials/02-first-search',
        'tutorials/03-two-player-games',
        'tutorials/04-solving-games',
        'tutorials/05-stochastic-games',
        'tutorials/06-neural-network-priors',
        'tutorials/07-advanced-search',
      ],
    },
    {
      type: 'category',
      label: 'How-To Guides',
      items: [
        'how-to/parallel-search',
        'how-to/tree-reuse',
        'how-to/progressive-widening',
        'how-to/batched-evaluation',
        'how-to/hyperparameter-tuning',
        'how-to/custom-tree-policy',
        'how-to/wasm-integration',
      ],
    },
    {
      type: 'category',
      label: 'Concepts',
      items: [
        'concepts/algorithm',
        'concepts/exploration-exploitation',
        'concepts/tree-policies',
        'concepts/solver-and-bounds',
        'concepts/chance-nodes',
        'concepts/parallel-mcts',
        'concepts/architecture',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      items: [
        'reference/traits',
        'reference/configuration',
        'reference/glossary',
      ],
    },
  ],
};

export default sidebars;
