# Widget Architecture Framework (WAF) - Twister v0.5+

**Purpose**: Define modular diorama-based widget system where each conductor track is an independently pluggable scene/environment (like Valve/Activision game dioramas).

**Vision**: Dorothy, the agentic intelligence layer, orchestrates these widgets dynamically. Adding a new widget (GPS, temperature, humidity, etc.) should NOT break the main application.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Twister Widget Engine                       │
│                  (Dorothy Orchestration Layer)                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Track A      │  │ Track B      │  │ Track C      │  ...     │
│  │ SIREN Diorama│  │ Signal Flow  │  │ Patterns &   │          │
│  │ (Real-time   │  │ Diorama      │  │ Dioramas     │          │
│  │  detection)  │  │ (Oscilloscope│  │ (Analytics)  │          │
│  │              │  │  spectrum)   │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Track D      │  │ Track E      │  │ Track F      │  ...     │
│  │ 3D Spatial   │  │ Knowledge    │  │ Material     │          │
│  │ Diorama      │  │ Graph        │  │ Properties   │          │
│  │ (PointCloud) │  │ Diorama      │  │ Diorama      │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Track G      │  │ Track VI     │  │ Track H      │  ...     │
│  │ Dorothy      │  │ Aether Viz   │  │ Haptic       │          │
│  │ Orchestrator │  │ Diorama      │  │ Diorama      │          │
│  │ (AG-UI Gen)  │  │ (RF-BSDF)    │  │ (DualSense)  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │         AG-UI/A2UI Protocol Layer                        │  │
│  │  (Dynamic UI Generation + Widget Composition)           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Slint Building Blocks (Reusable UI Components)         │  │
│  │  - Panel, Button, Slider, Graph, Heatmap, etc.         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Concept: Dioramas as Pluggable Widgets

### What is a "Diorama"?

In game design (Valve, Activision), a diorama is a **self-contained, immersive scene** with:
- Its own visual aesthetic (materials, lighting, colors)
- Its own interaction model (camera, controls)
- Its own data model (what it displays)
- Independence from other scenes (adding a new scene doesn't break existing ones)

**In Twister**: Each conductor track is a diorama. Examples:

| Track | Diorama Name | Purpose |
|-------|--------------|---------|
| A | SIREN Detection Scene | Real-time RF/audio detection (frequency waterfall, oscilloscope) |
| B | Signal Flow Scene | Time-domain audio flow, FFT energy, V-buffer history |
| C | Pattern Library Scene | 23 motifs as interactive cards, temporal frequency graphs |
| D | 3D Spatial Scene | Point cloud of attack sources, elevation/azimuth, time-scrub |
| E | Knowledge Graph Scene | Graph of events, patterns, frequencies as interactive nodes |
| F | Material Properties Scene | RF-BSDF visualization, hardness/roughness/wetness heatmaps |
| VI | Aether Wavefield Scene | 3D volumetric RF wavefield, Gaussian splatting, 360° view |
| H | Haptic Feedback Scene | DualSense controller haptic texture preview, waveform synthesizer |
| **GPS (future)** | **Location Scene** | Map of attack locations, satellite coverage, movement tracking |
| **Weather (future)** | **Environmental Scene** | Temperature, humidity, wind (context for RF propagation) |

---

## Widget System Design

### 1. Widget Registry (Runtime Discovery)

Each diorama registers itself with the widget engine:

```rust
// src/widget_registry.rs

pub trait WidgetDiorama: Send + Sync {
    /// Unique identifier (e.g., "siren", "spatial-3d", "gps")
    fn id(&self) -> &'static str;

    /// Human-readable name ("SIREN Detection Scene")
    fn name(&self) -> &'static str;

    /// Description for Dorothy's context
    fn description(&self) -> &'static str;

    /// List of data inputs this widget needs
    fn required_data(&self) -> Vec<DataRequirement>;

    /// Generate AG-UI component tree for this diorama
    fn generate_ui(&self, context: &WidgetContext) -> Result<AgUiNode, Box<dyn Error>>;

    /// Handle user interaction (clicks, drags, controller input)
    fn handle_event(&mut self, event: WidgetEvent) -> Result<(), Box<dyn Error>>;

    /// Update internal state from new data
    fn update(&mut self, data: &DataSnapshot) -> Result<(), Box<dyn Error>>;
}

pub struct WidgetRegistry {
    widgets: HashMap<String, Arc<dyn WidgetDiorama>>,
}

impl WidgetRegistry {
    pub fn register(&mut self, widget: Arc<dyn WidgetDiorama>) {
        self.widgets.insert(widget.id().to_string(), widget);
    }

    pub fn list_all(&self) -> Vec<(&str, &str)> {
        // Returns [(id, name), ...] for Dorothy to list available dioramas
        self.widgets
            .iter()
            .map(|(id, w)| (id.as_str(), w.name()))
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn WidgetDiorama>> {
        self.widgets.get(id)
    }
}
```

### 2. AG-UI Integration (Dynamic Composition)

Dorothy doesn't hardcode UI. She requests it dynamically:

```rust
// src/ag_ui_handler.rs

pub struct AgUiNode {
    pub component: String,      // "Panel", "Button", "Graph", "Heatmap", etc.
    pub properties: serde_json::Value,
    pub children: Vec<AgUiNode>,
    pub event_handlers: Vec<(String, String)>, // [(event, handler_name), ...]
}

pub struct WidgetContext {
    pub widget_id: String,
    pub user_query: String,     // What Dorothy is trying to show
    pub data: DataSnapshot,     // Current forensic/pattern/spatial data
    pub feature_flags: FeatureFlags,
}

impl WidgetContext {
    pub fn generate_ui_for_widget(
        &self,
        registry: &WidgetRegistry,
    ) -> Result<AgUiNode, Box<dyn Error>> {
        let widget = registry
            .get(&self.widget_id)
            .ok_or("Widget not found")?;

        widget.generate_ui(self)
    }
}

// Dorothy uses AG-UI to construct UI dynamically
pub async fn dorothy_show_widget(
    registry: &WidgetRegistry,
    query: &str,  // e.g., "show me the GPS diorama"
) -> Result<AgUiNode, Box<dyn Error>> {
    // 1. LFM-2.5 parses query to identify widget
    let widget_id = parse_widget_request(query)?;

    // 2. Fetch current data
    let data = fetch_current_forensic_snapshot().await?;

    // 3. Create context
    let context = WidgetContext {
        widget_id: widget_id.clone(),
        user_query: query.to_string(),
        data,
        feature_flags: FeatureFlags::all_enabled(),
    };

    // 4. Request AG-UI tree from widget
    let ui_tree = context.generate_ui_for_widget(registry)?;

    // 5. Render to Slint via AG-UI protocol
    render_ag_ui_in_slint(&ui_tree).await?;

    Ok(ui_tree)
}
```

### 3. Slint Building Blocks (Component Library)

Slint components serve as the **atomic building blocks** that AG-UI assembles:

```slint
// ui/building_blocks/common.slint

component Panel {
    in property title: string;
    in property background_color: color;
    in property width: length;
    in property height: length;

    Rectangle {
        width: parent.width;
        height: parent.height;
        background: parent.background_color;
    }

    Text {
        x: 10px;
        y: 5px;
        text: parent.title;
        color: white;
        font-size: 14px;
    }
}

component Graph {
    in property title: string;
    in property data_points: [float];
    in property x_label: string;
    in property y_label: string;

    // Internal: SVG rendering logic
    // AG-UI will inject data_points dynamically
}

component Heatmap {
    in property title: string;
    in property matrix: [[float]];
    in property color_map: string; // "viridis", "plasma", etc.
    in property min_value: float;
    in property max_value: float;
}
```

AG-UI assembles these into complete dioramas:

```json
// Example AG-UI node tree (generated by Dorothy)
{
  "component": "Panel",
  "properties": {
    "title": "3D Spatial Attack Sources",
    "background_color": "#1a1a2e"
  },
  "children": [
    {
      "component": "Graph",
      "properties": {
        "title": "Azimuth Distribution",
        "data_points": [0.1, 0.3, 0.5, ...],
        "x_label": "Azimuth (degrees)",
        "y_label": "Event Density"
      }
    },
    {
      "component": "Heatmap",
      "properties": {
        "title": "Elevation vs Frequency",
        "matrix": [[1.0, 0.8], [0.6, 0.9]],
        "color_map": "plasma",
        "min_value": 0.0,
        "max_value": 1.0
      }
    }
  ]
}
```

---

## Widget Independence (No Breaking Changes)

### Adding a New Widget (GPS Example)

```rust
// src/widgets/gps_widget.rs

pub struct GpsWidget {
    last_location: Option<GpsLocation>,
    satellite_count: u32,
}

impl WidgetDiorama for GpsWidget {
    fn id(&self) -> &'static str { "gps" }
    fn name(&self) -> &'static str { "GPS Location Scene" }
    fn description(&self) -> &'static str {
        "Display attack source locations on interactive map"
    }

    fn required_data(&self) -> Vec<DataRequirement> {
        vec![
            DataRequirement::PointCloud,  // 3D spatial events
            DataRequirement::Timestamps,
        ]
    }

    fn generate_ui(&self, context: &WidgetContext) -> Result<AgUiNode, Box<dyn Error>> {
        // Return AG-UI tree for map view
        Ok(AgUiNode {
            component: "Panel".to_string(),
            properties: json!({
                "title": "GPS Location Map",
                "width": "600px",
                "height": "400px",
            }),
            children: vec![
                AgUiNode {
                    component: "InteractiveMap".to_string(),
                    properties: json!({
                        "zoom": 10,
                        "center_lat": 37.7749,
                        "center_lon": -122.4194,
                    }),
                    children: vec![],
                    event_handlers: vec![],
                },
            ],
            event_handlers: vec![
                ("click".to_string(), "on_location_clicked".to_string()),
                ("drag".to_string(), "on_pan".to_string()),
            ],
        })
    }

    fn handle_event(&mut self, event: WidgetEvent) -> Result<(), Box<dyn Error>> {
        match event {
            WidgetEvent::Click(x, y) => {
                eprintln!("[GPS] User clicked at ({}, {})", x, y);
                // Query nearby events
            }
            _ => {}
        }
        Ok(())
    }

    fn update(&mut self, data: &DataSnapshot) -> Result<(), Box<dyn Error>> {
        self.last_location = data.spatial_centroid;
        self.satellite_count = data.satellite_count;
        Ok(())
    }
}
```

**Key point**: GPS widget is **completely independent**. It doesn't modify Track A/B/C/D/E code. It just:
1. Implements `WidgetDiorama` trait
2. Registers itself: `registry.register(Arc::new(GpsWidget::new()))`
3. Generates its own AG-UI tree
4. Updates from the shared `DataSnapshot`

---

## Dorothy's Role in Widget Orchestration

```rust
// src/ai/dorothy_orchestrator.rs

pub struct DorothyOrchestrator {
    widget_registry: WidgetRegistry,
    current_context: WidgetContext,
}

impl DorothyOrchestrator {
    pub async fn handle_user_request(&mut self, query: &str) -> Result<String, Box<dyn Error>> {
        // 1. Parse intent (using LFM-2.5)
        let intent = self.parse_intent(query).await?;

        match intent {
            Intent::ShowWidget(widget_id) => {
                // "Show me the GPS scene"
                self.show_widget(&widget_id).await?;
                Ok(format!("Displaying {} scene...", widget_id))
            }
            Intent::AnalyzePattern(pattern_id) => {
                // "Analyze pattern 7"
                self.switch_to_pattern_widget(pattern_id).await?;
                Ok(format!("Analyzing pattern {}", pattern_id))
            }
            Intent::QuerySpatial(region) => {
                // "Show me attacks on my face"
                self.switch_to_spatial_widget().await?;
                let points = self.filter_points_by_region(&region).await?;
                Ok(format!("Found {} events in {}", points.len(), region))
            }
            Intent::ExploreTimeline(time_range) => {
                // "Play back last week's attacks"
                self.show_temporal_widget(time_range).await?;
                Ok("Playing timeline...".to_string())
            }
        }
    }

    async fn show_widget(&mut self, widget_id: &str) -> Result<(), Box<dyn Error>> {
        // Update context to point to this widget
        self.current_context.widget_id = widget_id.to_string();

        // Fetch fresh data
        self.current_context.data = fetch_current_forensic_snapshot().await?;

        // Request AG-UI from the widget
        let ui_tree = self.current_context.generate_ui_for_widget(&self.widget_registry)?;

        // Render in Slint
        render_ag_ui_in_slint(&ui_tree).await?;

        Ok(())
    }
}
```

---

## Data Flow: Shared DataSnapshot

All widgets read from a shared, constantly-updated **DataSnapshot**:

```rust
// src/data/snapshot.rs

pub struct DataSnapshot {
    // From Track A (device state)
    pub detected_frequency: f32,
    pub detected_azimuth: f32,
    pub detected_elevation: f32,

    // From Track B (signal state)
    pub audio_rms: f32,
    pub rf_power: f32,
    pub vbuffer_frames: Vec<Vec<f32>>,

    // From Track C (patterns)
    pub active_pattern_id: Option<usize>,
    pub pattern_confidence: f32,
    pub pattern_frequency_hz: f32,

    // From Track D (spatial)
    pub point_cloud: Vec<Point3D>,
    pub spatial_centroid: Option<GpsLocation>,
    pub elevation_histogram: Vec<f32>,

    // From Track E (knowledge)
    pub graph_nodes: usize,
    pub graph_edges: usize,

    // From Track F (materials)
    pub active_material: Option<MaterialClassification>,
    pub hardness: f32,
    pub roughness: f32,
    pub wetness: f32,

    // From Track VI (wavefield)
    pub gaussian_splat_points: usize,
    pub wavefield_energy: f32,

    // From Track H (haptic)
    pub haptic_frequency: f32,
    pub haptic_intensity: f32,

    // Metadata
    pub timestamp: Instant,
}

// Dorothy and all widgets subscribe to updates:
pub async fn watch_data_snapshot() -> broadcast::Receiver<DataSnapshot> {
    // Returns a channel that broadcasts whenever DataSnapshot changes
}
```

---

## Widget Lifecycle

```
┌────────────────────────────────────────────────────────┐
│                  Widget Lifecycle                      │
├────────────────────────────────────────────────────────┤
│                                                        │
│  1. CREATION                                           │
│     └─ Widget instance created                         │
│     └─ Registers with WidgetRegistry                   │
│                                                        │
│  2. IDLE                                               │
│     └─ Listening for user requests                     │
│     └─ Updating from DataSnapshot (background)        │
│                                                        │
│  3. ACTIVE (User shows widget)                         │
│     └─ generate_ui() called                            │
│     └─ AG-UI tree rendered in Slint                    │
│     └─ Receiving user input (clicks, drags, etc.)      │
│     └─ handle_event() called                           │
│     └─ Synchronizing with DataSnapshot changes         │
│                                                        │
│  4. BACKGROUND (User switches to another widget)       │
│     └─ Paused rendering, but still updating state      │
│     └─ Ready to re-activate quickly                    │
│                                                        │
│  5. CLEANUP                                            │
│     └─ Deregister from registry                        │
│     └─ Clean up resources                              │
│                                                        │
└────────────────────────────────────────────────────────┘
```

---

## Widget Communication (Pub-Sub)

Widgets don't directly call each other. They use a publish-subscribe pattern:

```rust
// src/events/widget_events.rs

pub enum WidgetEvent {
    /// User selected a pattern in pattern widget
    PatternSelected { pattern_id: usize, confidence: f32 },

    /// User clicked a point in spatial widget
    SpatialPointSelected { azimuth: f32, elevation: f32 },

    /// User changed time range in temporal widget
    TimeRangeSelected { start: Instant, end: Instant },

    /// Dorothy requested widget to filter its display
    ApplyFilter { filter_key: String, filter_value: String },
}

pub struct WidgetEventBus {
    tx: broadcast::Sender<WidgetEvent>,
}

impl WidgetEventBus {
    pub fn publish(&self, event: WidgetEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WidgetEvent> {
        self.tx.subscribe()
    }
}

// In widgets:
pub async fn listen_for_pattern_selection(mut rx: broadcast::Receiver<WidgetEvent>) {
    while let Ok(event) = rx.recv().await {
        if let WidgetEvent::PatternSelected { pattern_id, .. } = event {
            eprintln!("[Widget] User selected pattern {}", pattern_id);
            // Update this widget's display based on the selection
        }
    }
}
```

---

## File Structure

```
src/
├── widgets/
│   ├── mod.rs                      # Widget trait + registry
│   ├── siren_widget.rs             # Track A diorama
│   ├── signal_flow_widget.rs       # Track B diorama
│   ├── patterns_widget.rs          # Track C diorama
│   ├── spatial_3d_widget.rs        # Track D diorama
│   ├── knowledge_graph_widget.rs   # Track E diorama
│   ├── material_widget.rs          # Track F diorama
│   ├── aether_wavefield_widget.rs  # Track VI diorama
│   ├── haptic_widget.rs            # Track H diorama
│   └── gps_widget.rs               # Future: independent GPS widget
│
├── ag_ui/
│   ├── mod.rs                      # AG-UI protocol handler
│   ├── node.rs                     # AgUiNode tree structure
│   ├── renderer.rs                 # Render AG-UI to Slint
│   └── composition.rs              # Widget composition logic
│
├── ai/
│   ├── dorothy_orchestrator.rs     # Main widget orchestrator
│   ├── intent_parser.rs            # LFM-2.5 intent recognition
│   └── widget_queries.rs           # Dorothy's widget-aware MCP tools
│
├── data/
│   ├── snapshot.rs                 # Shared DataSnapshot
│   └── widget_events.rs            # WidgetEvent pub-sub
│
└── main.rs                         # Initialize widget system
```

---

## Success Criteria

✅ **Widget Registry functional**: All tracks register as dioramas
✅ **AG-UI generation working**: Widgets generate UI trees dynamically
✅ **Slint integration**: AG-UI renders in Slint without hardcoding
✅ **Independent widgets**: Adding GPS widget doesn't break existing code
✅ **Dorothy orchestration**: Conversational requests map to widget displays
✅ **Data synchronization**: All widgets stay in sync via DataSnapshot
✅ **Event bus working**: Widgets communicate without coupling
✅ **No breaking changes**: Existing tracks untouched, only wrapped as widgets

---

## Next Steps

1. **Refactor Track G**: Make Dorothy the orchestrator (not just chat)
2. **Create Widget trait**: All tracks implement WidgetDiorama
3. **Build AG-UI protocol handler**: Connect to ag-ui-protocol spec
4. **Implement DataSnapshot**: Centralized, shared state
5. **Create example widget**: GPS or temperature (demonstrates independence)
6. **Test widget hot-swapping**: Dynamically register/unregister widgets at runtime

