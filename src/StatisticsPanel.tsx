import React, { useState, useEffect, useMemo } from 'react';
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';
import { t } from './i18n';
import { Activity, Clock } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

type Period = '24h' | '1w' | '2w' | 'all';

interface PlaySession {
  start: number;
  duration: number;
}

interface PlaytimeStats {
  sessions: PlaySession[];
}

const calculateOverlapHours = (sessions: PlaySession[], startPeriod: number, endPeriod: number) => {
  let val = 0;
  for (const s of sessions) {
    const sEnd = s.start + s.duration;
    const overlapStart = Math.max(s.start, startPeriod);
    const overlapEnd = Math.min(sEnd, endPeriod);
    if (overlapStart < overlapEnd) {
      val += (overlapEnd - overlapStart) / 3600;
    }
  }
  return val;
};

const generateRealData = (period: Period, sessions: PlaySession[]) => {
  const data = [];
  const now = Math.floor(Date.now() / 1000);
  
  if (period === '24h') {
    for (let i = 0; i < 24; i++) {
        const hourStart = now - (24 - i) * 3600;
        const hourEnd = hourStart + 3600;
        const val = calculateOverlapHours(sessions, hourStart, hourEnd);
        data.push({ name: `${24 - i}h`, value: Number(val.toFixed(1)) });
    }
  } else if (period === '1w') {
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    for (let i = 0; i < 7; i++) {
        const dayStart = now - (7 - i) * 86400;
        const dayEnd = dayStart + 86400;
        const val = calculateOverlapHours(sessions, dayStart, dayEnd);
        const d = new Date(dayStart * 1000);
        data.push({ name: days[d.getDay()], value: Number(val.toFixed(1)) });
    }
  } else if (period === '2w') {
    for (let i = 0; i < 14; i++) {
        const dayStart = now - (14 - i) * 86400;
        const dayEnd = dayStart + 86400;
        const val = calculateOverlapHours(sessions, dayStart, dayEnd);
        const d = new Date(dayStart * 1000);
        data.push({ name: `${d.getDate()}/${d.getMonth() + 1}`, value: Number(val.toFixed(1)) });
    }
  } else {
    for (let i = 0; i < 30; i++) {
        const dayStart = now - (30 - i) * 86400;
        const dayEnd = dayStart + 86400;
        const val = calculateOverlapHours(sessions, dayStart, dayEnd);
        const d = new Date(dayStart * 1000);
        data.push({ name: `${d.getDate()}/${d.getMonth() + 1}`, value: Number(val.toFixed(1)) });
    }
  }
  return data;
};

export default function StatisticsPanel() {
  const [period, setPeriod] = useState<Period>('2w');
  const [stats, setStats] = useState<PlaytimeStats>({ sessions: [] });

  useEffect(() => {
    invoke<PlaytimeStats>('get_playtime_stats').then((res) => {
        setStats(res);
    }).catch(console.error);
  }, []);

  const data = useMemo(() => generateRealData(period, stats.sessions), [period, stats.sessions]);

  const totalPlaytimeHours = useMemo(() => {
    const totalSecs = stats.sessions.reduce((acc, s) => acc + s.duration, 0);
    return Number((totalSecs / 3600).toFixed(1));
  }, [stats]);

  const twoWeeksPlaytimeHours = useMemo(() => {
    const now = Math.floor(Date.now() / 1000);
    const twoWeeksAgo = now - 14 * 86400;
    const val = calculateOverlapHours(stats.sessions, twoWeeksAgo, now);
    return Number(val.toFixed(1));
  }, [stats]);

  const CustomTooltip = ({ active, payload, label }: any) => {
    if (active && payload && payload.length) {
      return (
        <div className="stats-tooltip">
          <p className="stats-tooltip-label">{label}</p>
          <p className="stats-tooltip-value">
            {payload[0].value} {t().hoursShort}
          </p>
        </div>
      );
    }
    return null;
  };

  return (
    <div className="statistics-panel-container">
      <div className="modpack-page-header" style={{ marginBottom: 16 }}>
        <h2 className="modpack-page-title" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {t().statistics}
        </h2>
      </div>

      <div className="statistics-content">
        <div className="stats-cards-row">
          <div className="stat-card">
            <div className="stat-card-icon">
              <Clock size={20} />
            </div>
            <div className="stat-card-info">
              <span className="stat-card-label">{t().totalPlaytime}</span>
              <span className="stat-card-value">{totalPlaytimeHours} <span className="stat-card-unit">{t().hoursShort}</span></span>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-card-icon">
              <Activity size={20} />
            </div>
            <div className="stat-card-info">
              <span className="stat-card-label">{t().playtimeTwoWeeks}</span>
              <span className="stat-card-value">{twoWeeksPlaytimeHours} <span className="stat-card-unit">{t().hoursShort}</span></span>
            </div>
          </div>
        </div>

        <div className="stats-chart-section">
          <div className="stats-chart-header">
            <div className="stats-period-selector">
              <button className={`period-btn ${period === '24h' ? 'active' : ''}`} onClick={() => setPeriod('24h')}>{t().period24h}</button>
              <button className={`period-btn ${period === '1w' ? 'active' : ''}`} onClick={() => setPeriod('1w')}>{t().period1w}</button>
              <button className={`period-btn ${period === '2w' ? 'active' : ''}`} onClick={() => setPeriod('2w')}>{t().period2w}</button>
              <button className={`period-btn ${period === 'all' ? 'active' : ''}`} onClick={() => setPeriod('all')}>{t().periodAllTime}</button>
            </div>
          </div>

          <div className="stats-chart-container">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
                <defs>
                  <linearGradient id="colorValue" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="var(--accent)" stopOpacity={0.4} />
                    <stop offset="95%" stopColor="var(--accent)" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <XAxis
                  dataKey="name"
                  axisLine={false}
                  tickLine={false}
                  tick={{ fill: '#888', fontSize: 12 }}
                  dy={10}
                />
                <YAxis
                  axisLine={false}
                  tickLine={false}
                  tick={{ fill: '#888', fontSize: 12 }}
                  dx={-10}
                />
                <Tooltip content={<CustomTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)', strokeWidth: 1, strokeDasharray: '3 3' }} />
                <Area
                  type="monotone"
                  dataKey="value"
                  stroke="var(--accent)"
                  strokeWidth={2}
                  fillOpacity={1}
                  fill="url(#colorValue)"
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>
    </div>
  );
}
