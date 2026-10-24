import type {ReactNode} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import HomepageFeatures from '@site/src/components/HomepageFeatures';
import DashboardHeroCarousel from '@site/src/components/DashboardHeroCarousel';
import Heading from '@theme/Heading';

import styles from './index.module.css';

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary hero-bg', styles.heroBanner)} style={{ minHeight: '85vh', display: 'flex', alignItems: 'center', position: 'relative' }}>
      
      {/* Animated background particles */}
      <div className="particles-container">
        <div className="particle"></div>
        <div className="particle"></div>
        <div className="particle"></div>
        <div className="particle"></div>
        <div className="particle"></div>
      </div>

      <div className="container" style={{ position: 'relative', zIndex: 2 }}>
        <div style={{ maxWidth: '900px', margin: '0 auto', textAlign: 'center' }}>
          
          <div className="logo-container">
            <img src="img/AMUD-logo.png" alt="AMUD Logo" className="floating-logo" />
            <div className="logo-glow"></div>
          </div>

          <Heading as="h1" className="hero__title" style={{ color: '#ffffff', fontSize: '5.5rem', fontWeight: 900, lineHeight: 1.1, marginBottom: '0.5rem', letterSpacing: '-0.02em' }}>
            Unify Your <span className="gradient-text">Homelab.</span>
          </Heading>
          
          <h2 style={{ color: '#ff6b2b', fontSize: '2.5rem', fontWeight: 800, textTransform: 'uppercase', letterSpacing: '0.1em', marginBottom: '1.5rem', textShadow: '0 0 20px rgba(255, 107, 43, 0.4)' }}>
            AMUD Dashboard
          </h2>

          <p className="hero__subtitle" style={{ color: '#a0aabf', fontSize: '1.5rem', marginBottom: '3.5rem', fontWeight: 400, maxWidth: '700px', margin: '0 auto 3.5rem auto' }}>
            {siteConfig.tagline}
          </p>
          
          <div className={styles.buttons} style={{ display: 'flex', gap: '2rem', justifyContent: 'center' }}>
            <Link
              className="button button--lg glow-button pulse-btn"
              to="/docs/intro">
              <span style={{ position: 'relative', zIndex: 2 }}>🚀 Explore the Docs</span>
            </Link>
            <Link
              className="button button--lg outline-button"
              to="https://github.com/boubli/AMUD-Dashboard">
              View on GitHub
            </Link>
          </div>
        </div>
        
        <div className="dashboard-preview-container">
          <div className="mac-window-bar">
            <div className="mac-dot close"></div>
            <div className="mac-dot minimize"></div>
            <div className="mac-dot maximize"></div>
          </div>
          <DashboardHeroCarousel />
        </div>
      </div>
      
      {/* Bottom wave separator */}
      <div className="hero-wave">
        <svg data-name="Layer 1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 120" preserveAspectRatio="none" aria-hidden="true" focusable="false">
          <path d="M321.39,56.44c58-10.79,114.16-30.13,172-41.86,82.39-16.72,168.19-17.73,250.45-.39C823.78,31,906.67,72,985.66,92.83c70.05,18.48,146.53,26.09,214.34,3V120H0V95.8C-1,95.8,11,94,22.86,92.17c60.36-9.42,120.91-23.82,181.74-32.91C238.29,51.81,280.12,59.39,321.39,56.44Z" className="shape-fill"></path>
        </svg>
      </div>
    </header>
  );
}

export default function Home(): ReactNode {
  return (
    <Layout
      title={`Home`}
      description="AMUD Ecosystem Landing Page">
      <HomepageHeader />
      <main style={{ background: '#0b0e14', position: 'relative', zIndex: 1 }}>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
