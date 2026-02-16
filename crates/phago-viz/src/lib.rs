//! # Phago Viz
//!
//! Self-contained HTML visualization for Phago colonies.
//!
//! Generates a single HTML file with embedded D3.js that shows:
//! - Knowledge graph (force-directed network)
//! - Agent canvas (2D spatial view)
//! - Event timeline
//! - Metrics dashboard with tick slider

use phago_core::types::Tick;
use phago_runtime::colony::{ColonyEvent, ColonySnapshot};

/// Generate a self-contained HTML file with D3.js visualization.
///
/// The HTML embeds all data as JSON constants and loads D3.js from CDN.
/// No server, no npm — just open the file in a browser.
pub fn generate_html(snapshots: &[ColonySnapshot], events: &[(Tick, ColonyEvent)]) -> String {
    let snapshots_json = serde_json::to_string(snapshots).unwrap_or_else(|_| "[]".to_string());
    let events_json = serde_json::to_string(events).unwrap_or_else(|_| "[]".to_string());

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Phago Colony Visualization</title>
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{ background: #1a1a2e; color: #e0e0e0; font-family: 'Courier New', monospace; overflow: hidden; }}
.container {{ display: grid; grid-template-columns: 1fr 1fr 280px; grid-template-rows: 1fr 200px 60px; height: 100vh; gap: 2px; background: #0f0f23; }}
.panel {{ background: #1a1a2e; border: 1px solid #333366; border-radius: 4px; overflow: hidden; position: relative; }}
.panel-title {{ position: absolute; top: 4px; left: 8px; font-size: 11px; color: #7777aa; text-transform: uppercase; letter-spacing: 1px; z-index: 10; }}
#graph-panel {{ grid-column: 1; grid-row: 1; }}
#agent-panel {{ grid-column: 2; grid-row: 1; }}
#sidebar {{ grid-column: 3; grid-row: 1 / 3; padding: 12px; overflow-y: auto; }}
#timeline-panel {{ grid-column: 1 / 3; grid-row: 2; }}
#controls {{ grid-column: 1 / 4; grid-row: 3; display: flex; align-items: center; padding: 8px 16px; gap: 16px; }}
#tick-slider {{ flex: 1; accent-color: #5555ff; }}
#tick-label {{ font-size: 14px; color: #aaaadd; min-width: 100px; }}
.stat-row {{ display: flex; justify-content: space-between; padding: 4px 0; border-bottom: 1px solid #222244; font-size: 12px; }}
.stat-label {{ color: #8888bb; }}
.stat-value {{ color: #ddddff; font-weight: bold; }}
.section-title {{ color: #9999cc; font-size: 11px; text-transform: uppercase; letter-spacing: 1px; margin: 12px 0 6px 0; }}
.legend {{ display: flex; flex-wrap: wrap; gap: 8px; margin: 8px 0; }}
.legend-item {{ display: flex; align-items: center; gap: 4px; font-size: 10px; }}
.legend-dot {{ width: 8px; height: 8px; border-radius: 50%; }}
svg {{ width: 100%; height: 100%; }}
.node-label {{ font-size: 9px; fill: #aaa; pointer-events: none; }}
.tooltip {{ position: absolute; background: #222244; border: 1px solid #444488; padding: 6px 10px; border-radius: 4px; font-size: 11px; pointer-events: none; z-index: 100; display: none; }}
h2 {{ font-size: 14px; color: #bbbbee; margin-bottom: 8px; }}
#play-btn {{ background: #333366; border: 1px solid #5555aa; color: #ddddff; padding: 6px 16px; border-radius: 4px; cursor: pointer; font-family: inherit; }}
#play-btn:hover {{ background: #444488; }}
</style>
</head>
<body>
<div class="container">
  <div class="panel" id="graph-panel">
    <div class="panel-title">Knowledge Graph</div>
    <svg id="graph-svg"></svg>
  </div>
  <div class="panel" id="agent-panel">
    <div class="panel-title">Agent Canvas</div>
    <svg id="agent-svg"></svg>
  </div>
  <div class="panel" id="sidebar">
    <h2>Phago Colony</h2>
    <div class="section-title">Agents</div>
    <div class="legend">
      <div class="legend-item"><div class="legend-dot" style="background:#44cc44"></div> Digester</div>
      <div class="legend-item"><div class="legend-dot" style="background:#cc4444"></div> Sentinel</div>
      <div class="legend-item"><div class="legend-dot" style="background:#aa44cc"></div> Synthesizer</div>
    </div>
    <div class="section-title">Graph Nodes</div>
    <div class="legend">
      <div class="legend-item"><div class="legend-dot" style="background:#4488cc"></div> Concept</div>
      <div class="legend-item"><div class="legend-dot" style="background:#ccaa22"></div> Insight</div>
      <div class="legend-item"><div class="legend-dot" style="background:#cc4444"></div> Anomaly</div>
    </div>
    <div class="section-title">Metrics</div>
    <div id="metrics-panel">
      <div class="stat-row"><span class="stat-label">Tick</span><span class="stat-value" id="m-tick">0</span></div>
      <div class="stat-row"><span class="stat-label">Nodes</span><span class="stat-value" id="m-nodes">0</span></div>
      <div class="stat-row"><span class="stat-label">Edges</span><span class="stat-value" id="m-edges">0</span></div>
      <div class="stat-row"><span class="stat-label">Agents Alive</span><span class="stat-value" id="m-agents">0</span></div>
      <div class="stat-row"><span class="stat-label">Docs Digested</span><span class="stat-value" id="m-docs">0</span></div>
    </div>
    <div class="section-title">Events</div>
    <div id="event-counts">
      <div class="stat-row"><span class="stat-label">Transfers</span><span class="stat-value" id="m-transfers">0</span></div>
      <div class="stat-row"><span class="stat-label">Integrations</span><span class="stat-value" id="m-integrations">0</span></div>
      <div class="stat-row"><span class="stat-label">Symbioses</span><span class="stat-value" id="m-symbioses">0</span></div>
      <div class="stat-row"><span class="stat-label">Dissolutions</span><span class="stat-value" id="m-dissolutions">0</span></div>
      <div class="stat-row"><span class="stat-label">Deaths</span><span class="stat-value" id="m-deaths">0</span></div>
    </div>
  </div>
  <div class="panel" id="timeline-panel">
    <div class="panel-title">Event Timeline</div>
    <svg id="timeline-svg"></svg>
  </div>
  <div id="controls">
    <button id="play-btn">&#9654; Play</button>
    <input type="range" id="tick-slider" min="0" max="0" value="0">
    <span id="tick-label">Tick 0 / 0</span>
  </div>
</div>
<div class="tooltip" id="tooltip"></div>

<script src="https://d3js.org/d3.v7.min.js"></script>
<script>
const SNAPSHOTS = {snapshots};
const EVENTS = {events};

if (SNAPSHOTS.length === 0) {{
  document.body.innerHTML = '<div style="padding:40px;color:#888">No snapshots recorded.</div>';
}}

const slider = document.getElementById('tick-slider');
const tickLabel = document.getElementById('tick-label');
const playBtn = document.getElementById('play-btn');
const tooltip = document.getElementById('tooltip');

slider.max = SNAPSHOTS.length - 1;
let currentIdx = SNAPSHOTS.length - 1;
slider.value = currentIdx;
let playing = false;
let playInterval = null;

function showTooltip(text, x, y) {{
  tooltip.style.display = 'block';
  tooltip.textContent = text;
  tooltip.style.left = (x + 10) + 'px';
  tooltip.style.top = (y - 20) + 'px';
}}
function hideTooltip() {{ tooltip.style.display = 'none'; }}

// --- Knowledge Graph ---
const graphSvg = d3.select('#graph-svg');
const graphG = graphSvg.append('g');
let graphSim = null;

function updateGraph(snap) {{
  const width = document.getElementById('graph-panel').clientWidth;
  const height = document.getElementById('graph-panel').clientHeight;

  const nodeMap = {{}};
  snap.nodes.forEach((n, i) => {{ nodeMap[n.label] = i; n.index = i; }});

  const links = snap.edges.filter(e => nodeMap[e.from_label] !== undefined && nodeMap[e.to_label] !== undefined)
    .map(e => ({{ source: nodeMap[e.from_label], target: nodeMap[e.to_label], weight: e.weight, co_activations: e.co_activations }}));

  const nodeColor = d => {{
    if (d.node_type === 'Insight') return '#ccaa22';
    if (d.node_type === 'Anomaly') return '#cc4444';
    return '#4488cc';
  }};

  // Links
  const link = graphG.selectAll('line.graph-link').data(links, (d,i) => i);
  link.exit().remove();
  const linkEnter = link.enter().append('line').attr('class', 'graph-link');
  const linkAll = linkEnter.merge(link)
    .attr('stroke', '#334466').attr('stroke-opacity', d => Math.min(d.weight, 0.8))
    .attr('stroke-width', d => Math.max(d.weight * 2, 0.5));

  // Nodes
  const node = graphG.selectAll('circle.graph-node').data(snap.nodes, d => d.label);
  node.exit().remove();
  const nodeEnter = node.enter().append('circle').attr('class', 'graph-node')
    .on('mouseover', (ev, d) => showTooltip(`${{d.label}} (${{d.node_type}}) access:${{d.access_count}}`, ev.pageX, ev.pageY))
    .on('mouseout', hideTooltip);
  const nodeAll = nodeEnter.merge(node)
    .attr('r', d => Math.max(3, Math.min(d.access_count * 1.5, 15)))
    .attr('fill', nodeColor).attr('opacity', 0.85);

  // Labels
  const label = graphG.selectAll('text.node-label').data(snap.nodes, d => d.label);
  label.exit().remove();
  const labelEnter = label.enter().append('text').attr('class', 'node-label');
  const labelAll = labelEnter.merge(label).text(d => d.label);

  if (graphSim) graphSim.stop();
  graphSim = d3.forceSimulation(snap.nodes)
    .force('link', d3.forceLink(links).distance(60))
    .force('charge', d3.forceManyBody().strength(-40))
    .force('center', d3.forceCenter(width / 2, height / 2))
    .on('tick', () => {{
      linkAll.attr('x1', d => d.source.x).attr('y1', d => d.source.y)
             .attr('x2', d => d.target.x).attr('y2', d => d.target.y);
      nodeAll.attr('cx', d => d.x).attr('cy', d => d.y);
      labelAll.attr('x', d => d.x + 8).attr('y', d => d.y + 3);
    }});
}}

// --- Agent Canvas ---
const agentSvg = d3.select('#agent-svg');

function updateAgents(snap) {{
  const width = document.getElementById('agent-panel').clientWidth;
  const height = document.getElementById('agent-panel').clientHeight;

  // Compute scale from agent positions
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  snap.agents.forEach(a => {{
    minX = Math.min(minX, a.position.x); maxX = Math.max(maxX, a.position.x);
    minY = Math.min(minY, a.position.y); maxY = Math.max(maxY, a.position.y);
  }});
  const pad = 40;
  const rangeX = Math.max(maxX - minX, 1);
  const rangeY = Math.max(maxY - minY, 1);
  const scaleX = d => pad + (d.position.x - minX) / rangeX * (width - 2 * pad);
  const scaleY = d => pad + (d.position.y - minY) / rangeY * (height - 2 * pad);

  const agentColor = d => {{
    if (d.agent_type === 'digester') return '#44cc44';
    if (d.agent_type === 'sentinel') return '#cc4444';
    if (d.agent_type === 'synthesizer') return '#aa44cc';
    return '#888888';
  }};

  const circ = agentSvg.selectAll('circle.agent').data(snap.agents, d => d.id.toString());
  circ.exit().transition().duration(200).attr('r', 0).remove();
  const circEnter = circ.enter().append('circle').attr('class', 'agent')
    .attr('r', 0)
    .on('mouseover', (ev, d) => showTooltip(`${{d.agent_type}} age:${{d.age}} perm:${{d.permeability.toFixed(2)}} vocab:${{d.vocabulary_size}}`, ev.pageX, ev.pageY))
    .on('mouseout', hideTooltip);
  circEnter.merge(circ).transition().duration(300)
    .attr('cx', scaleX).attr('cy', scaleY)
    .attr('r', 10)
    .attr('fill', agentColor)
    .attr('opacity', d => 0.3 + (1.0 - d.permeability) * 0.7)
    .attr('stroke', '#ffffff22').attr('stroke-width', 1);

  // Labels
  const lbl = agentSvg.selectAll('text.agent-label').data(snap.agents, d => d.id.toString());
  lbl.exit().remove();
  const lblEnter = lbl.enter().append('text').attr('class', 'agent-label')
    .attr('font-size', '9px').attr('fill', '#888');
  lblEnter.merge(lbl).transition().duration(300)
    .attr('x', d => scaleX(d) + 12).attr('y', d => scaleY(d) + 3)
    .text(d => d.agent_type.slice(0, 3));
}}

// --- Timeline ---
const timelineSvg = d3.select('#timeline-svg');

function initTimeline() {{
  const width = document.getElementById('timeline-panel').clientWidth;
  const height = document.getElementById('timeline-panel').clientHeight;
  const pad = {{ left: 40, right: 20, top: 25, bottom: 20 }};

  if (EVENTS.length === 0) return;

  const maxTick = Math.max(...EVENTS.map(e => e[0]));
  const x = d3.scaleLinear().domain([0, maxTick]).range([pad.left, width - pad.right]);

  // Color by event type
  const eventColor = e => {{
    const t = e[1];
    if (t.CapabilityExported) return '#4488cc';
    if (t.CapabilityIntegrated) return '#44aacc';
    if (t.Symbiosis) return '#44cc44';
    if (t.Dissolved) return '#ccaa22';
    if (t.Died) return '#222222';
    if (t.Presented) return '#666688';
    return '#444444';
  }};

  // Y jitter by type
  const eventY = e => {{
    const t = e[1];
    if (t.CapabilityExported || t.CapabilityIntegrated) return 0.2;
    if (t.Symbiosis) return 0.4;
    if (t.Dissolved) return 0.6;
    if (t.Died) return 0.8;
    return 0.5;
  }};

  const significant = EVENTS.filter(e => {{
    const t = e[1];
    return t.CapabilityExported || t.CapabilityIntegrated || t.Symbiosis || t.Dissolved || t.Died;
  }});

  const yScale = d3.scaleLinear().domain([0, 1]).range([pad.top, height - pad.bottom]);

  timelineSvg.selectAll('circle.event-dot').data(significant)
    .enter().append('circle').attr('class', 'event-dot')
    .attr('cx', d => x(d[0]))
    .attr('cy', d => yScale(eventY(d)) + (Math.random() - 0.5) * 10)
    .attr('r', 3)
    .attr('fill', eventColor)
    .attr('opacity', 0.7)
    .on('mouseover', (ev, d) => {{
      const t = d[1];
      const type = Object.keys(t)[0] || 'Event';
      showTooltip(`Tick ${{d[0]}}: ${{type}}`, ev.pageX, ev.pageY);
    }})
    .on('mouseout', hideTooltip);

  // Tick cursor line
  timelineSvg.append('line').attr('id', 'tick-cursor')
    .attr('y1', pad.top).attr('y2', height - pad.bottom)
    .attr('stroke', '#ff5555').attr('stroke-width', 1.5).attr('opacity', 0.6);

  // Axis
  timelineSvg.append('g').attr('transform', `translate(0,${{height - pad.bottom}})`)
    .call(d3.axisBottom(x).ticks(10)).selectAll('text,line,path').attr('stroke', '#555577').attr('fill', '#555577');

  // Legend
  const legendData = [
    ['Transfer', '#4488cc'], ['Symbiosis', '#44cc44'],
    ['Dissolution', '#ccaa22'], ['Death', '#222222']
  ];
  const lg = timelineSvg.append('g').attr('transform', `translate(${{width - 200}}, 8)`);
  legendData.forEach((d, i) => {{
    lg.append('circle').attr('cx', i * 50).attr('cy', 0).attr('r', 4).attr('fill', d[1]);
    lg.append('text').attr('x', i * 50 + 7).attr('y', 3).text(d[0]).attr('fill', '#888').attr('font-size', '9px');
  }});
}}

function updateTickCursor(snap) {{
  const width = document.getElementById('timeline-panel').clientWidth;
  const pad = {{ left: 40, right: 20 }};
  if (EVENTS.length === 0) return;
  const maxTick = Math.max(...EVENTS.map(e => e[0]));
  const x = d3.scaleLinear().domain([0, maxTick]).range([pad.left, width - pad.right]);
  d3.select('#tick-cursor').attr('x1', x(snap.tick)).attr('x2', x(snap.tick));
}}

// --- Metrics ---
function updateMetrics(snap) {{
  document.getElementById('m-tick').textContent = snap.tick;
  document.getElementById('m-nodes').textContent = snap.stats.graph_nodes;
  document.getElementById('m-edges').textContent = snap.stats.graph_edges;
  document.getElementById('m-agents').textContent = snap.stats.agents_alive;
  document.getElementById('m-docs').textContent = snap.stats.documents_digested + ' / ' + snap.stats.documents_total;

  // Count events up to this tick
  let transfers = 0, integrations = 0, symbioses = 0, dissolutions = 0, deaths = 0;
  EVENTS.forEach(e => {{
    if (e[0] > snap.tick) return;
    const t = e[1];
    if (t.CapabilityExported) transfers++;
    if (t.CapabilityIntegrated) integrations++;
    if (t.Symbiosis) symbioses++;
    if (t.Dissolved) dissolutions++;
    if (t.Died) deaths++;
  }});
  document.getElementById('m-transfers').textContent = transfers;
  document.getElementById('m-integrations').textContent = integrations;
  document.getElementById('m-symbioses').textContent = symbioses;
  document.getElementById('m-dissolutions').textContent = dissolutions;
  document.getElementById('m-deaths').textContent = deaths;
}}

// --- Update all panels ---
function update(idx) {{
  if (idx < 0 || idx >= SNAPSHOTS.length) return;
  currentIdx = idx;
  slider.value = idx;
  const snap = SNAPSHOTS[idx];
  tickLabel.textContent = `Tick ${{snap.tick}} / ${{SNAPSHOTS[SNAPSHOTS.length - 1].tick}}`;
  updateGraph(snap);
  updateAgents(snap);
  updateTickCursor(snap);
  updateMetrics(snap);
}}

// --- Controls ---
slider.addEventListener('input', () => {{
  update(parseInt(slider.value));
}});

playBtn.addEventListener('click', () => {{
  if (playing) {{
    playing = false;
    clearInterval(playInterval);
    playBtn.innerHTML = '&#9654; Play';
  }} else {{
    playing = true;
    playBtn.innerHTML = '&#9646;&#9646; Pause';
    if (currentIdx >= SNAPSHOTS.length - 1) currentIdx = 0;
    playInterval = setInterval(() => {{
      if (currentIdx >= SNAPSHOTS.length - 1) {{
        playing = false;
        clearInterval(playInterval);
        playBtn.innerHTML = '&#9654; Play';
        return;
      }}
      update(currentIdx + 1);
    }}, 500);
  }}
}});

// --- Init ---
initTimeline();
update(SNAPSHOTS.length - 1);
</script>
</body>
</html>"##,
        snapshots = snapshots_json,
        events = events_json,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use phago_core::types::*;
    use phago_runtime::colony::{AgentSnapshot, ColonySnapshot, ColonyStats, NodeSnapshot};

    #[test]
    fn html_contains_required_elements() {
        let snapshot = ColonySnapshot {
            tick: 10,
            agents: vec![AgentSnapshot {
                id: AgentId::new(),
                agent_type: "digester".to_string(),
                position: Position::new(1.0, 2.0),
                age: 10,
                permeability: 0.3,
                vocabulary_size: 5,
            }],
            nodes: vec![NodeSnapshot {
                id: NodeId::new(),
                label: "cell".to_string(),
                node_type: NodeType::Concept,
                position: Position::new(0.0, 0.0),
                access_count: 3,
            }],
            edges: vec![],
            stats: ColonyStats {
                tick: 10,
                agents_alive: 1,
                agents_died: 0,
                total_spawned: 1,
                graph_nodes: 1,
                graph_edges: 0,
                total_signals: 0,
                documents_total: 1,
                documents_digested: 1,
            },
        };

        let html = generate_html(&[snapshot], &[]);
        assert!(html.contains("<html"), "should contain html tag");
        assert!(html.contains("d3.v7"), "should reference D3 v7");
        assert!(html.contains("SNAPSHOTS"), "should embed snapshot data");
        assert!(html.contains("EVENTS"), "should embed event data");
        assert!(html.contains("digester"), "should contain agent data");
    }

    #[test]
    fn html_empty_data_does_not_panic() {
        let html = generate_html(&[], &[]);
        assert!(
            html.contains("<html"),
            "should produce valid html even with empty data"
        );
    }
}
