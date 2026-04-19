import clsx from 'clsx';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

import styles from './index.module.css';

const guideCards = [
  {
    title: 'Get Started',
    to: '/docs/getting-started',
    body: 'Create a client, pick the right published Rust crate, and understand the workspace layering model.',
  },
  {
    title: 'Check Coverage',
    to: '/docs/api-coverage',
    body: 'See which Alpaca market data and trading HTTP endpoints are implemented across the direct HTTP crates.',
  },
  {
    title: 'Use the API Reference',
    to: '/docs/reference',
    body: 'Open crate guides, resource guides, and the published rustdoc entry points for each Rust crate.',
  },
  {
    title: 'Project Layout',
    to: '/docs/project-structure',
    body: 'See the workspace source tree, crate boundaries, release layering, and the path-versus-package naming map.',
  },
];

const resourceCards = [
  {title: 'alpaca-core', to: '/docs/reference/alpaca-core', body: 'Shared primitives, credentials, URL helpers, and common typed foundations.'},
  {title: 'alpaca-rest-http', to: '/docs/reference/alpaca-rest-http', body: 'Shared HTTP transport, retry behavior, observers, and response metadata handling.'},
  {title: 'alpaca-data', to: '/docs/reference/alpaca-data', body: 'Market data client coverage for stocks, options, news, and corporate actions.'},
  {title: 'alpaca-trade', to: '/docs/reference/alpaca-trade', body: 'Trading client coverage for account, assets, orders, positions, activities, and watchlists.'},
  {title: 'alpaca-mock', to: '/docs/reference/alpaca-mock', body: 'Executable mock server flows for trade lifecycle verification and market-data-backed simulations.'},
  {title: 'alpaca-time', to: '/docs/reference/alpaca-time', body: 'New York time, trading-calendar, expiration, and display semantics shared across the Rust workspace.'},
  {title: 'alpaca-option', to: '/docs/reference/alpaca-option', body: 'Provider-neutral option contracts, snapshots, pricing, payoff, and URL helpers.'},
  {title: 'alpaca-facade', to: '/docs/reference/alpaca-facade', body: 'High-level convenience adapters that compose the lower workspace crates.'},
];

export default function Home(): JSX.Element {
  return (
    <Layout
      title="Documentation"
      description="Public documentation and generated API reference for the Rust workspace that publishes Alpaca HTTP SDKs, time semantics, option models, and convenience facades."
    >
      <main>
        <section className={styles.hero}>
          <div className="container">
            <div className={styles.heroGrid}>
              <div>
                <p className={styles.kicker}>Rust + Alpaca HTTP + Market Semantics</p>
                <h1 className={styles.title}>Rust Workspace for Alpaca HTTP SDKs, Time Semantics, and Option Models</h1>
                <p className={styles.subtitle}>
                  Typed async access to Alpaca market data and trading HTTP APIs, plus reusable time semantics,
                  provider-neutral option models, and a high-level convenience facade built on top of the lower layers.
                </p>
                <div className={styles.actions}>
                  <Link className="button button--primary button--lg" to="/docs/getting-started">
                    Read the docs
                  </Link>
                  <Link className="button button--secondary button--lg" to="/docs/reference">
                    Browse API reference
                  </Link>
                </div>
              </div>
              <div className={styles.heroPanel}>
                <div className={styles.panelLabel}>Workspace contract</div>
                <ul className={styles.panelList}>
                  <li>The official Alpaca HTTP API defines endpoint semantics.</li>
                  <li>The published Rust surface is split into foundation, semantic, and facade layers.</li>
                  <li>Primary direct HTTP entry points are alpaca-data and alpaca-trade.</li>
                  <li>alpaca-time and alpaca-option carry the reusable semantic core.</li>
                  <li>Optional TypeScript companions are plus features, not the primary published surface.</li>
                </ul>
              </div>
            </div>
          </div>
        </section>

        <section className={styles.section}>
          <div className="container">
            <div className={styles.sectionHeader}>
              <h2>Core Guides</h2>
              <p>Start with workspace behavior, auth rules, and repository layout before jumping into crate and resource pages.</p>
            </div>
            <div className={styles.cardGrid}>
              {guideCards.map((card) => (
                <Link key={card.title} className={clsx(styles.card, styles.guideCard)} to={card.to}>
                  <h3>{card.title}</h3>
                  <p>{card.body}</p>
                </Link>
              ))}
            </div>
          </div>
        </section>

        <section className={clsx(styles.section, styles.resourceSection)}>
          <div className="container">
            <div className={styles.sectionHeader}>
              <h2>Published Rust Crates</h2>
              <p>Each crate page links the public guide to the corresponding published rustdoc and shows where that crate fits in the workspace layers.</p>
            </div>
            <div className={styles.cardGrid}>
              {resourceCards.map((card) => (
                <Link key={card.title} className={clsx(styles.card, styles.resourceCard)} to={card.to}>
                  <h3>{card.title}</h3>
                  <p>{card.body}</p>
                </Link>
              ))}
            </div>
          </div>
        </section>
      </main>
    </Layout>
  );
}
