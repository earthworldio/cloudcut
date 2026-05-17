import React, { useMemo } from 'react';
import { useUIStore } from '../../stores/uiStore';

export const TimelineRuler: React.FC = () => {
  const zoomLevel = useUIStore((state) => state.zoomLevel);
  
  /* 
    zoomLevel is pixels per second.
    We want tick marks every 1 second, and major marks every 5 seconds.
  */
  const ticks = useMemo(() => {
    const items = [];
    const totalSeconds = 3600; /* 1 hour of ruler */
    
    for (let i = 0; i <= totalSeconds; i++) {
      const x = i * zoomLevel;
      const isMajor = i % 5 === 0;
      
      items.push(
        <div 
          key={i} 
          className={`absolute bottom-0 border-l ${isMajor ? 'h-4 border-muted-foreground' : 'h-2 border-muted'}`}
          style={{ left: x }}
        >
          {isMajor && (
            <span className="absolute bottom-4 left-1 text-[10px] text-muted-foreground whitespace-nowrap">
              {formatTime(i * 1000)}
            </span>
          )}
        </div>
      );
    }
    return items;
  }, [zoomLevel]);

  return (
    <div className="relative h-10 w-[100000px] bg-muted/30 border-b border-border">
      {ticks}
    </div>
  );
};

function formatTime(ms: number) {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const ss = (s % 60).toString().padStart(2, '0');
  const mm = (m % 60).toString().padStart(2, '0');
  const hh = h.toString().padStart(2, '0');
  return `${hh}:${mm}:${ss}`;
}
