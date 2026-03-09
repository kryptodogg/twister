import re

with open("ui/app.slint", "r") as f:
    content = f.read()

# Add properties
if "in property <string> gate-status:" not in content:
    replacement = """    in property <string> note-name:   "---";   // e.g. "A4", "C#5"
    in property <float> note-cents:  0.0;     // cents offset from raw detection

    in property <string> gate-status: "IDLE";
    in property <string> last-gate-reason: "";
    in property <int> training-pairs-dropped: 0;
    in property <int> gate-rejections-low-anomaly: 0;
    in property <int> gate-rejections-low-confidence: 0;"""
    content = content.replace('    in property <string> note-name:   "---";   // e.g. "A4", "C#5"\n    in property <float> note-cents:  0.0;     // cents offset from raw detection', replacement)

# Add Gate UI
if "GATE STATUS" not in content:
    gate_ui = """                    // GATE STATUS
                    Lbl { text: "GATE STATUS"; }
                    HorizontalLayout {
                        spacing: 8px;
                        Mon {
                            text: gate-status;
                            color: gate-status == "FORWARD" ? Pal.green : gate-status == "REJECTED" ? Pal.red : Pal.lo;
                            font-size: 14px;
                        }
                    }
                    Mon {
                        text: last-gate-reason;
                        color: Pal.lo;
                        font-size: 9px;
                        wrap: word-wrap;
                    }

                    Rectangle { height: 8px; }

                    Lbl { text: "QUEUE DIAGNOSTICS"; }
                    HorizontalLayout {
                        spacing: 8px;
                        VerticalLayout {
                            Lbl { text: "DROPPED"; }
                            Mon { text: training-pairs-dropped; color: training-pairs-dropped > 0 ? Pal.red : Pal.lo; font-size: 10px; }
                        }
                        VerticalLayout {
                            Lbl { text: "REJ(ANOM)"; }
                            Mon { text: gate-rejections-low-anomaly; color: Pal.lo; font-size: 10px; }
                        }
                        VerticalLayout {
                            Lbl { text: "REJ(CONF)"; }
                            Mon { text: gate-rejections-low-confidence; color: Pal.lo; font-size: 10px; }
                        }
                    }
                    Rectangle { height: 8px; }"""

    content = content.replace("                    // Pairs indicator", gate_ui + "\n                    // Pairs indicator")

with open("ui/app.slint", "w") as f:
    f.write(content)
