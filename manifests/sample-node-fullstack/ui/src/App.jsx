import React, { useState, useEffect } from 'react';

export default function App() {
  const [count, setCount] = useState(0);
  const [loading, setLoading] = useState(true);

  const fetchCount = async () => {
    try {
      const res = await fetch('/api/count');
      const data = await res.json();
      setCount(data.count);
    } catch (e) {
      console.error('Failed to fetch count', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCount();
  }, []);

  const handleAction = async (action) => {
    setLoading(true);
    try {
      const res = await fetch(`/api/${action}`, { method: 'POST' });
      const data = await res.json();
      setCount(data.count);
    } catch (e) {
      console.error(`Failed to ${action}`, e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{
      minHeight: '100vh',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      padding: '20px'
    }}>
      <div style={{
        background: 'white',
        padding: '40px',
        borderRadius: '12px',
        boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)',
        textAlign: 'center',
        maxWidth: '400px',
        width: '100%'
      }}>
        <h1 style={{ margin: '0 0 10px 0', fontSize: '24px' }}>🔢 Sample Counter</h1>
        <p style={{ color: '#6b7280', margin: '0 0 30px 0' }}>
          Running inside the <b>AI Launcher</b> sandbox.<br/>
          (React + Express Fullstack)
        </p>

        <div style={{
          fontSize: '48px',
          fontWeight: 'bold',
          background: '#f3f4f6',
          borderRadius: '8px',
          padding: '20px',
          marginBottom: '30px'
        }}>
          {loading ? '...' : count}
        </div>

        <div style={{ display: 'flex', gap: '10px', justifyContent: 'center' }}>
          <button 
            disabled={loading}
            onClick={() => handleAction('decrement')}
            style={{
              padding: '10px 15px',
              border: 'none',
              borderRadius: '6px',
              background: '#e5e7eb',
              color: '#374151',
              cursor: loading ? 'not-allowed' : 'pointer',
              fontWeight: '500'
            }}
          >
            ➖ Decrement
          </button>
          
          <button 
            disabled={loading}
            onClick={() => handleAction('reset')}
            style={{
              padding: '10px 15px',
              border: 'none',
              borderRadius: '6px',
              background: '#fef3c7',
              color: '#92400e',
              cursor: loading ? 'not-allowed' : 'pointer',
              fontWeight: '500'
            }}
          >
            🔄 Reset
          </button>

          <button 
            disabled={loading}
            onClick={() => handleAction('increment')}
            style={{
              padding: '10px 15px',
              border: 'none',
              borderRadius: '6px',
              background: '#3b82f6',
              color: 'white',
              cursor: loading ? 'not-allowed' : 'pointer',
              fontWeight: '500'
            }}
          >
            ➕ Increment
          </button>
        </div>
      </div>
    </div>
  );
}
