import clsx from 'clsx';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

import styles from './index.module.css';

const guideCards = [
  {
    title: 'Get Started',
    to: '/docs/getting-started',
    body: 'Create a client, pick the public crate entry point, and understand the workspace layering model.',
  },
  {
    title: 'Check Coverage',
    to: '/docs/api-coverage',
    body: 'See which Alpaca market data and trading HTTP endpoints are implemented across the public crates.',
  },
  {
    title: 'Use the API Reference',
    to: '/docs/reference',
    body: 'Open crate guides, resource guides, and the published rustdoc entry points for each public package.',
  },
  {
    title: 'Project Layout',
    to: '/docs/project-structure',
    body: 'See the workspace source tree, crate boundaries, release layering, and mock-server placement.',
  },
];

const resourceCards = [
  {title: 'alpaca-data', to: '/docs/reference/alpaca-data', body: 'Market data client coverage for stocks, options, news, and corporate actions.'},
  {title: 'alpaca-trade', to: '/docs/reference/alpaca-trade', body: 'Trading client coverage for account, assets, orders, positions, activities, and watchlists.'},
  {title: 'alpaca-core', to: '/docs/reference/alpaca-core', body: 'Shared primitives, credentials, URL helpers, and common typed foundations.'},
  {title: 'alpaca-rest-http', to: '/docs/reference/alpaca-rest-http', body: 'Shared HTTP transport, retry behavior, observers, and response metadata handling.'},
  {title: 'alpaca-mock', to: '/docs/reference/alpaca-mock', body: 'Executable mock server flows for trade lifecycle verification and market-data-backed simulations.'},
];

export default function Home(): JSX.Element {
  return (
    <Layout
      title="Documentation"
      description="Public documentation and generated API reference for the high-performance Rust client for Alpaca market data and trading APIs."
    >
      <main>
        <section className={styles.hero}>
          <div className="container">
            <div className={styles.heroGrid}>
              <div>
                <p className={styles.kicker}>Rust + Alpaca Market Data + Trading</p>
                <h1 className={styles.title}>High-Performance Rust Client for Alpaca Market Data and Trading APIs</h1>
                <p className={styles.subtitle}>
                  Typed async access to Alpaca market data and trading HTTP APIs, split across public crates for
                  market data, trading, shared transport, shared primitives, and the executable mock server.
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
                  <li>Primary public entry points are alpaca-data and alpaca-trade.</li>
                  <li>Shared foundations live in alpaca-core and alpaca-rest-http.</li>
                  <li>alpaca-mock stays executable-first for trade lifecycle verification.</li>
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
              <h2>API Modules</h2>
              <p>Each crate page links the public guide to the corresponding published rustdoc and the resource-level reference pages inside this workspace.</p>
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
