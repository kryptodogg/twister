import React, { useState, useMemo, useRef } from 'react';

export default function App() {
  return (
    <div className="min-h-screen bg-neutral-900 flex items-center justify-center p-8 font-mono selection:bg-[#00E5C8] selection:text-black">
      <ChronosWidget />
    </div>
  );
}

const ChronosWidget = () => {
  // --- STATE ---
  // Temperature controls the edge density of the visualization
  const [temperature, setTemperature] = useState(0.14);
  const sliderRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);

  // --- INTERACTION ---
  const handlePointerDown = (e: React.PointerEvent) => {
    setIsDragging(true);
    updateTemperatureFromEvent(e);
    // Capture pointer to track dragging outside the element
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (isDragging) {
      updateTemperatureFromEvent(e);
    }
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    setIsDragging(false);
    e.currentTarget.releasePointerCapture(e.pointerId);
  };

  const updateTemperatureFromEvent = (e: React.PointerEvent) => {
    if (!sliderRef.current) return;
    const rect = sliderRef.current.getBoundingClientRect();
    let newX = e.clientX - rect.left;
    newX = Math.max(0, Math.min(newX, rect.width));
    const newTemp = Number((newX / rect.width).toFixed(2));
    setTemperature(newTemp);
  };

  // --- GRAPH DATA GENERATION ---
  // Nodes stay stable
  const numNodes = 45;
  const canvasWidth = 324; // 336 total - 2*6 padding = 324
  const canvasHeight = 110;

  const nodes = useMemo(() => {
    const generatedNodes = [];
    for (let i = 0; i < numNodes; i++) {
      const progress = i / (numNodes - 1);
      
      // Keep x within bounds (padding 10px on each side)
      const x = 15 + progress * (canvasWidth - 30);
      
      // Scatter Y around a center axis (y=45)
      // Base sine wave for a manifold structure + random jitter
      const yOffset = Math.sin(progress * Math.PI * 5) * 12;
      // We use a pseudo-random generator seeded by index so it's stable
      const jitter = ((Math.sin(i * 12.9898) * 43758.5453) % 1) * 20 - 10;
      const y = 45 + yOffset + jitter;

      let color = '#00E5C8'; // Teal (Past)
      let opacity = 1;

      if (progress > 0.66) {
        // Future/Predicted - Red fading to dim red
        color = '#FF4444'; 
        if (progress > 0.8) {
          const fade = 1 - ((progress - 0.8) / 0.2);
          opacity = 0.2 + (fade * 0.8);
        }
      } else if (progress > 0.33) {
        // Transition/Now - Violet
        color = '#A855F7'; 
      }

      generatedNodes.push({ id: i, x, y, color, opacity, progress });
    }
    return generatedNodes;
  }, [canvasWidth]);

  // Edges depend on temperature to show real-time changes
  const edges = useMemo(() => {
    const generatedEdges = [];
    // Base edges: sequential connections
    for (let i = 0; i < numNodes - 1; i++) {
      generatedEdges.push({ source: nodes[i], target: nodes[i + 1] });
      
      // Temperature driven connections: higher temp = more connections spanning further
      const baseProbability = temperature * 0.8; 
      
      if (i < numNodes - 2 && (Math.sin(i * 45.1) % 1 + 1) / 2 < baseProbability + 0.2) {
        generatedEdges.push({ source: nodes[i], target: nodes[i + 2] });
      }
      
      if (i < numNodes - 3 && (Math.sin(i * 92.3) % 1 + 1) / 2 < baseProbability - 0.1) {
        generatedEdges.push({ source: nodes[i], target: nodes[i + 3] });
      }

      // High temp causes long-range random connections (wormholes)
      if (temperature > 0.5 && (Math.sin(i * 11.2) % 1 + 1) / 2 < (temperature - 0.5)) {
        const targetIdx = Math.min(numNodes - 1, i + Math.floor(4 + ((Math.sin(i*3)%1+1)/2) * 5));
        generatedEdges.push({ source: nodes[i], target: nodes[targetIdx] });
      }
    }
    return generatedEdges;
  }, [nodes, temperature]);

  // --- LOSS SPARKLINE DATA ---
  const sparklinePoints = useMemo(() => {
    const points = [];
    let currentY = 10;
    // Generate ~20 points
    for (let i = 0; i <= 20; i++) {
      currentY += (Math.sin(i * 87.2) % 1) * 8 - 3.5; // Jagged downward trend
      currentY = Math.max(2, Math.min(22, currentY));
      points.push(`${i * (100 / 20)},${currentY}`);
    }
    return points.join(' ');
  }, []);

  return (
    <div 
      className="w-[336px] h-[212px] bg-[#0F111A] rounded-[8px] p-[6px] flex flex-col box-border shadow-2xl shadow-black/80"
      style={{
        boxShadow: '0 24px 48px -12px rgba(0,0,0,0.9), inset 0 1px 0 rgba(255,255,255,0.06)',
        WebkitFontSmoothing: 'antialiased'
      }}
    >
      {/* --- TOP ZONE --- 
          19% height (~40px). Here we use absolute 38px + 2px margin = 40px
      */}
      <div className="h-[38px] w-full rounded-[6px] border border-[#2A2E3D] px-[6px] py-[4px] flex justify-between items-center mb-[4px] bg-[#0F111A] shrink-0">
        <div className="flex flex-col justify-center gap-[2px]">
          <div className="text-[#00E5C8] uppercase tracking-[0.15em] leading-none font-medium font-[Inter] text-[11px]">
            CHRONOS
          </div>
          <div className="text-[#9CA3AF] leading-none font-[Inter] text-[9px]">
            TimeGNN · Prediction Active
          </div>
        </div>

        {/* Temperature Control */}
        <div className="flex flex-col items-end justify-center gap-[4px] pr-[2px]">
          <div className="flex justify-between items-center w-[60px]">
            <span className="text-[#9CA3AF] uppercase leading-none tracking-wide font-[Inter] text-[9px] relative -left-[24px]">TEMPERATURE</span>
            <span className="text-[#00E5C8] leading-none font-medium font-[Inter] text-[9px] relative -left-[20px]">τ = {temperature.toFixed(2)}</span>
          </div>
          
          {/* Slider interactive area */}
          <div 
            className="w-[60px] h-[12px] -my-[4.5px] flex items-center cursor-ew-resize touch-none group"
            ref={sliderRef}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp}
            onPointerCancel={handlePointerUp}
          >
            {/* Slider Track */}
            <div className="w-full h-[3px] bg-[#0A0C14] rounded-full border border-[#2A2E3D] relative overflow-hidden group-hover:border-[#3A3E4D] transition-colors">
              {/* Fill */}
              <div 
                className="absolute top-0 bottom-0 left-0 bg-[#00E5C8] opacity-30"
                style={{ width: `${temperature * 100}%` }}
              />
              {/* Thumb */}
              <div 
                className="absolute top-0 bottom-0 w-[3px] bg-[#00E5C8] shadow-[0_0_4px_#00E5C8]"
                style={{ 
                  left: `${temperature * 100}%`,
                  transform: 'translateX(-50%)'
                }}
              />
            </div>
          </div>
        </div>
      </div>

      {/* --- MIDDLE ZONE --- 
          52% height (~110px). Canvas bg #0A0C14, 1pt border #2A2E3D, 5pt radius
      */}
      <div className="h-[110px] w-full rounded-[5px] border border-[#2A2E3D] bg-[#0A0C14] relative mb-[4px] shrink-0 overflow-hidden">
        {/* Corner Labels */}
        <div className="absolute top-[5px] left-[6px] text-[#00E5C8] leading-none z-10 font-medium text-[9px]">
          PREDICTION HORIZON
        </div>
        <div className="absolute top-[5px] right-[6px] text-[#9CA3AF] leading-none z-10 uppercase tracking-wide text-[9px]">
          DAY 42 · 97-DAY BUFFER
        </div>

        {/* Graph Render */}
        <svg className="absolute inset-0 w-full h-full pointer-events-none">
          <defs>
            <marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M 0 2 L 10 5 L 0 8 z" fill="#00E5C8" opacity="0.8" />
            </marker>
            {/* Glows */}
            <filter id="glowTeal" x="-50%" y="-50%" width="200%" height="200%">
              <feGaussianBlur stdDeviation="1.5" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <filter id="glowViolet" x="-50%" y="-50%" width="200%" height="200%">
              <feGaussianBlur stdDeviation="1.5" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <filter id="glowRed" x="-50%" y="-50%" width="200%" height="200%">
              <feGaussianBlur stdDeviation="1.5" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
          </defs>

          {/* Edges */}
          {edges.map((edge, i) => (
            <line 
              key={`edge-${i}`}
              x1={edge.source.x} 
              y1={edge.source.y} 
              x2={edge.target?.x} 
              y2={edge.target?.y} 
              stroke={edge.source.color} 
              strokeWidth="0.5" 
              opacity={edge.source.opacity * 0.4}
            />
          ))}

          {/* Nodes */}
          {nodes.map((node) => {
            let filter = '';
            if (node.progress <= 0.33) filter = 'url(#glowTeal)';
            else if (node.progress <= 0.66) filter = 'url(#glowViolet)';
            else filter = 'url(#glowRed)';

            return (
              <circle 
                key={`node-${node.id}`}
                cx={node.x} 
                cy={node.y} 
                r={node.progress > 0.66 && node.progress < 0.8 ? "2.5" : "2"} 
                fill={node.color}
                opacity={node.opacity}
                filter={filter}
              />
            );
          })}

          {/* Time Axis */}
          <g transform="translate(0, 92)">
            {/* Main axis line */}
            <line x1="12" y1="0" x2={canvasWidth - 12} y2="0" stroke="#00E5C8" strokeWidth="0.5" opacity="0.6" markerEnd="url(#arrow)" />
            {/* Ticks */}
            {[...Array(12)].map((_, i) => (
              <line 
                key={`tick-${i}`} 
                x1={15 + i * ((canvasWidth - 30) / 11)} 
                y1="-2" 
                x2={15 + i * ((canvasWidth - 30) / 11)} 
                y2="2" 
                stroke="#00E5C8" 
                strokeWidth="0.5" 
                opacity="0.6" 
              />
            ))}
          </g>
        </svg>

        {/* Axis Labels */}
        <div className="absolute bottom-[4px] left-[15px] text-[#9CA3AF] leading-none tracking-widest uppercase text-[8px] font-[Inter]">
          PAST
        </div>
        <div className="absolute bottom-[4px] left-1/2 transform -translate-x-1/2 text-[#00E5C8] leading-none tracking-widest uppercase text-[8px] font-[Inter]">
          NOW
        </div>
        <div className="absolute bottom-[4px] right-[20px] text-[#FF4444] leading-none tracking-widest uppercase opacity-70 text-[8px] font-[Inter]">
          PREDICTED
        </div>
      </div>

      {/* --- BOTTOM ZONE --- 
          29% height (~62px). Three tiles, 4pt gaps.
      */}
      <div className="h-[54px] w-full flex gap-[4px] shrink-0">
        {/* Tile 1: Loss */}
        <div className="flex-1 rounded-[4px] border border-[#2A2E3D] bg-[#0A0C14] p-[6px] flex flex-col justify-between relative overflow-hidden">
          <div className="text-[#A855F7] uppercase leading-none tracking-wide font-[Inter] text-[9px]">NT-XENT LOSS</div>
          <div className="text-white text-[12px] font-light leading-none z-10 mb-[6px]">0.34</div>
          
          <div className="absolute bottom-[4px] left-[6px] right-[6px] h-[20px]">
            <svg className="w-full h-full overflow-visible" viewBox="0 0 100 24" preserveAspectRatio="none">
              <polyline 
                points={sparklinePoints}
                fill="none" 
                stroke="#A855F7" 
                strokeWidth="1.2" 
                vectorEffect="non-scaling-stroke"
                strokeLinejoin="round"
                opacity="0.9"
              />
            </svg>
          </div>
        </div>

        {/* Tile 2: Motif */}
        <div className="flex-1 rounded-[4px] border border-[#2A2E3D] bg-[#0A0C14] p-[6px] flex flex-col relative overflow-hidden">
          <div className="text-[#00E5C8] uppercase leading-none tracking-wide mb-[3px] text-[9px] font-[Inter]">MOTIF</div>
          <div className="text-white leading-none mb-[3px] font-[Inter] text-[10px]">SPARKLE</div>
          <div className="text-[#9CA3AF] leading-none text-[9px] font-[Inter]">Phase 2 of 3</div>
          
          {/* Progress Bar (Teal, Left-to-Right) */}
          <div className="absolute bottom-0 left-0 w-full h-[2px] bg-[#2A2E3D]">
            <div className="h-full bg-[#00E5C8] w-[66%] shadow-[0_0_4px_#00E5C8]"></div>
          </div>
        </div>

        {/* Tile 3: Next Event */}
        <div className="flex-1 rounded-[4px] border border-[#2A2E3D] bg-[#0A0C14] p-[6px] flex flex-col relative overflow-hidden">
          <div className="text-[#FF4444] opacity-80 uppercase leading-none tracking-wide mb-[3px] font-[Inter] text-[9px]">NEXT EVENT</div>
          <div className="text-white font-light leading-none mb-[3px] font-[Inter] text-[11px]">+1m 47s</div>
          <div className="text-[#9CA3AF] leading-none whitespace-nowrap overflow-hidden text-ellipsis text-[9px] font-[Inter]">nRF24 burst · 71%</div>
          
          {/* Progress Bar (Red, Right-to-Left Countdown) */}
          <div className="absolute bottom-0 left-0 w-full h-[2px] bg-[#2A2E3D]">
            <div className="h-full bg-[#FF4444] w-[30%] absolute right-0 top-0 shadow-[0_0_4px_#FF4444]"></div>
          </div>
        </div>
      </div>
    </div>
  );
};