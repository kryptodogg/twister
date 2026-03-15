import { useSettings } from '../context/SettingsContext';
import { DeviceCard } from '../components/ui/DeviceCard';
import { MdSelect, MdSelectOption } from '../components/md3/Select';
import { MdTextField } from '../components/md3/TextField';
import { MdSwitch } from '../components/md3/Switch';
import { MdSlider } from '../components/md3/Slider';
import { MdFilledTonalButton } from '../components/md3/Button';

export function Hardware() {
  const { settings, updateDevice } = useSettings();

  if (!settings) return <div className="p-8 text-on-surface/50">Loading settings...</div>;

  const sdrSettings = settings.devices['rtl-sdr'] || {};
  const plutoSettings = settings.devices['pluto-sdr'] || {};
  const audioSettings = settings.devices['c925e-audio'] || {};
  const magSettings = settings.devices['telephone-coil'] || {};
  const ovSettings = settings.devices['ov9281-dual'] || {};
  const picoSettings = settings.devices['pico-2'] || {};

  return (
    <div className="flex flex-col gap-10 max-w-7xl mx-auto pb-20">
      <section>
        <div className="flex items-center gap-4 mb-6">
          <h2 className="text-xl font-bold tracking-tight uppercase">RF / Software Defined Radio</h2>
          <div className="flex-1 h-px bg-white/5"></div>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <DeviceCard deviceId="rtl-sdr" title="RTL-SDR + Youloop" accent="rx">
            <div className="flex flex-col gap-5">
              <div className="flex justify-between items-center text-xs">
                <span className="text-on-surface/50">MODE</span>
                <span className="font-bold text-rx">RX ONLY</span>
              </div>
              <div className="flex justify-between items-center text-xs">
                <span className="text-on-surface/50">FREQUENCY RANGE</span>
                <span className="font-mono">10 kHz – 300 MHz</span>
              </div>

              <MdSelect
                label="Sample Rate"
                value={sdrSettings.sample_rate}
                onSelected={(v) => updateDevice('rtl-sdr', { sample_rate: v })}
              >
                {['250k', '1M', '2M', '2.4M', '3.2M'].map(rate => (
                  <MdSelectOption key={rate} value={rate}>{rate} sps</MdSelectOption>
                ))}
              </MdSelect>

              <MdTextField
                label="PPM Correction"
                type="number"
                value={sdrSettings.ppm}
                onInput={(v) => updateDevice('rtl-sdr', { ppm: parseInt(v) })}
              />

              <div className="flex items-center justify-between">
                <div className="flex flex-col">
                  <span className="text-sm font-medium">Gain Mode</span>
                  <span className="text-[10px] text-on-surface/50 uppercase">{sdrSettings.gain_mode === 'manual' ? 'Manual' : 'Auto'}</span>
                </div>
                <MdSwitch
                  selected={sdrSettings.gain_mode === 'manual'}
                  onSelected={(sel) => updateDevice('rtl-sdr', { gain_mode: sel ? 'manual' : 'auto' })}
                />
              </div>

              {sdrSettings.gain_mode === 'manual' && (
                <div className="flex flex-col gap-2">
                  <div className="flex justify-between text-[10px] uppercase font-bold text-on-surface/50">
                    <span>Manual Gain</span>
                    <span>{sdrSettings.gain || 0} dB</span>
                  </div>
                  <MdSlider
                    min={0} max={50} value={sdrSettings.gain || 0}
                    onInput={(v) => updateDevice('rtl-sdr', { gain: v })}
                  />
                </div>
              )}

              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Youloop</span>
                <MdSwitch
                  selected={sdrSettings.youloop}
                  onSelected={(sel) => updateDevice('rtl-sdr', { youloop: sel })}
                />
              </div>
            </div>
          </DeviceCard>

          <DeviceCard deviceId="pluto-sdr" title="PlutoSDR + PA" accent="tx">
            <div className="flex flex-col gap-5">
              <MdSelect
                label="Mode"
                value={plutoSettings.mode}
                onSelected={(v) => updateDevice('pluto-sdr', { mode: v })}
              >
                <MdSelectOption value="rx">RX Only</MdSelectOption>
                <MdSelectOption value="tx">TX Only</MdSelectOption>
                <MdSelectOption value="full_duplex">Full Duplex</MdSelectOption>
              </MdSelect>

              <div className="flex justify-between items-center text-xs">
                <span className="text-on-surface/50 uppercase">Frequency Range</span>
                <span className="font-mono">70 MHz – 6 GHz</span>
              </div>

              {(plutoSettings.mode === 'tx' || plutoSettings.mode === 'full_duplex') && (
                <div className="flex flex-col gap-2">
                  <div className="flex justify-between text-[10px] uppercase font-bold text-tx">
                    <span>TX Power</span>
                    <span>{plutoSettings.tx_power || 0} dBm</span>
                  </div>
                  <MdSlider
                    min={0} max={75} value={plutoSettings.tx_power || 0}
                    onInput={(v) => updateDevice('pluto-sdr', { tx_power: v })}
                  />
                </div>
              )}

              {(plutoSettings.mode === 'rx' || plutoSettings.mode === 'full_duplex') && (
                <div className="flex flex-col gap-2">
                  <div className="flex justify-between text-[10px] uppercase font-bold text-rx">
                    <span>RX Gain</span>
                    <span>{plutoSettings.rx_gain || 0} dB</span>
                  </div>
                  <MdSlider
                    min={0} max={75} value={plutoSettings.rx_gain || 0}
                    onInput={(v) => updateDevice('pluto-sdr', { rx_gain: v })}
                  />
                </div>
              )}

              <MdSelect
                label="Sample Rate"
                value={plutoSettings.sample_rate}
                onSelected={(v) => updateDevice('pluto-sdr', { sample_rate: v })}
              >
                {['1M', '2M', '5M', '10M', '20M'].map(rate => (
                  <MdSelectOption key={rate} value={rate}>{rate} sps</MdSelectOption>
                ))}
              </MdSelect>

              <div className="p-3 rounded bg-white/5 border border-white/5 flex flex-col gap-1">
                <span className="text-[10px] text-on-surface/50 font-bold uppercase">Antenna Baseline</span>
                <span className="text-xs">~12mm (measure to confirm)</span>
              </div>
            </div>
          </DeviceCard>
        </div>
      </section>

      <md-divider></md-divider>

      <section>
        <div className="flex items-center gap-4 mb-6">
          <h2 className="text-xl font-bold tracking-tight uppercase">Audio and Magnetic</h2>
          <div className="flex-1 h-px bg-white/5"></div>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <DeviceCard deviceId="c925e-audio" title="C925e Audio" accent="rx">
             <div className="flex flex-col gap-5">
                <MdSelect
                  label="Mode"
                  value={audioSettings.mode}
                  onSelected={(v) => updateDevice('c925e-audio', { mode: v })}
                >
                  <MdSelectOption value="audio_rx">Audio RX</MdSelectOption>
                  <MdSelectOption value="visual_mic">Visual Microphone</MdSelectOption>
                  <MdSelectOption value="both">Both</MdSelectOption>
                </MdSelect>

                <MdSelect
                  label="Channels"
                  value={audioSettings.channels}
                  onSelected={(v) => updateDevice('c925e-audio', { channels: v })}
                >
                  <MdSelectOption value="stereo_l">Stereo L</MdSelectOption>
                  <MdSelectOption value="stereo_r">Stereo R</MdSelectOption>
                  <MdSelectOption value="both">Both</MdSelectOption>
                </MdSelect>

                <div className="flex items-center justify-between">
                  <div className="flex flex-col gap-1">
                    <span className="text-sm font-medium">Raw Mode</span>
                    {!audioSettings.raw_mode && (
                       <span className="text-[10px] px-1.5 py-0.5 rounded bg-br-tan/20 text-br-tan font-bold">PREPROCESSING ACTIVE — NOT FORENSIC</span>
                    )}
                  </div>
                  <MdSwitch
                    selected={audioSettings.raw_mode}
                    onSelected={(sel) => updateDevice('c925e-audio', { raw_mode: sel })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div className="flex flex-col">
                    <span className="text-sm font-medium">Visual Microphone</span>
                    {audioSettings.visual_mic_enabled && (
                      <span className="text-[10px] text-rx font-mono">32 kHz temporal sampling</span>
                    )}
                  </div>
                  <MdSwitch
                    selected={audioSettings.visual_mic_enabled}
                    onSelected={(sel) => updateDevice('c925e-audio', { visual_mic_enabled: sel })}
                  />
                </div>
             </div>
          </DeviceCard>

          <DeviceCard deviceId="telephone-coil" title="Telephone Coil → Realtek" accent="config">
            <div className="flex flex-col gap-5">
              <div className="p-3 rounded bg-white/5 border border-white/5 flex flex-col gap-1">
                <span className="text-[10px] text-on-surface/50 font-bold uppercase">Device</span>
                <span className="text-xs uppercase">ASUS TUF B550m — Realtek ALC1200</span>
              </div>

              <div className="p-3 rounded bg-br-tan/10 border border-br-tan/20 flex flex-col gap-1">
                <span className="text-xs font-bold text-br-tan uppercase">Differential Magnetometer</span>
                <span className="text-[10px] text-on-surface/70">NOT A MICROPHONE — Differential inductance monitoring.</span>
              </div>

              <div className="flex justify-between items-center text-xs">
                <span className="text-on-surface/50 uppercase">Effective Band</span>
                <span className="font-mono">1 Hz – 192 kHz</span>
              </div>

              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">60 Hz Powerline Monitoring</span>
                <MdSwitch
                  selected={magSettings.monitoring_60hz}
                  onSelected={(sel) => updateDevice('telephone-coil', { monitoring_60hz: sel })}
                />
              </div>

              <MdSelect
                label="Harmonic Tracking Depth"
                value={magSettings.harmonic_depth}
                onSelected={(v) => updateDevice('telephone-coil', { harmonic_depth: v })}
              >
                {['2nd', '3rd', '5th', '10th'].map(d => (
                  <MdSelectOption key={d} value={d}>{d}</MdSelectOption>
                ))}
              </MdSelect>

              <div className="text-[10px] text-on-surface/50 italic leading-relaxed">
                Super-Nyquist aliasing is intentional for raw signal reconstruction and neural training.
              </div>
            </div>
          </DeviceCard>
        </div>
      </section>

      <md-divider></md-divider>

      <section>
        <div className="flex items-center gap-4 mb-6">
          <h2 className="text-xl font-bold tracking-tight uppercase">Optical and Proximity</h2>
          <div className="flex-1 h-px bg-white/5"></div>
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
           <DeviceCard deviceId="ov9281-dual" title="OV9281 Dual Stereo" accent="rx">
              <div className="flex flex-col gap-5">
                 <div className="grid grid-cols-2 gap-4">
                    <div className="p-3 rounded bg-white/5 border border-white/5 text-center">
                       <span className="text-[10px] block text-on-surface/50 font-bold uppercase">Left Cam</span>
                       <span className="text-xs font-mono">2560×800</span>
                    </div>
                    <div className="p-3 rounded bg-white/5 border border-white/5 text-center">
                       <span className="text-[10px] block text-on-surface/50 font-bold uppercase">Right Cam</span>
                       <span className="text-xs font-mono">2560×800</span>
                    </div>
                 </div>

                 <MdSelect
                   label="FPS"
                   value={String(ovSettings.fps)}
                   onSelected={(v) => updateDevice('ov9281-dual', { fps: parseInt(v) })}
                 >
                   {[30, 60, 90, 120].map(fps => (
                     <MdSelectOption key={fps} value={String(fps)}>{fps} FPS</MdSelectOption>
                   ))}
                 </MdSelect>

                 <div className="flex justify-between items-center text-xs">
                    <span className="text-on-surface/50 uppercase">Shutter</span>
                    <span className="font-bold">GLOBAL</span>
                 </div>

                 <MdSelect
                   label="Mode"
                   value={ovSettings.mode}
                   onSelected={(v) => updateDevice('ov9281-dual', { mode: v })}
                 >
                   <MdSelectOption value="stereo_depth">Stereo Depth</MdSelectOption>
                   <MdSelectOption value="pose_estimation">Pose Estimation</MdSelectOption>
                   <MdSelectOption value="ir_detector">IR Detector</MdSelectOption>
                   <MdSelectOption value="all">All Modalities</MdSelectOption>
                 </MdSelect>

                 <MdTextField
                   label="Lens Focal Length (mm)"
                   type="number"
                   value={ovSettings.focal_length}
                   onInput={(v) => updateDevice('ov9281-dual', { focal_length: parseFloat(v) })}
                 />

                 <div className="flex items-center justify-between">
                    <div className="flex flex-col gap-1">
                       <span className="text-[10px] font-bold text-on-surface/50 uppercase">Calibration</span>
                       <span className="text-xs text-br-green font-bold">VALIDATED</span>
                    </div>
                    <MdFilledTonalButton>RECALIBRATE (ChArUco)</MdFilledTonalButton>
                 </div>

                 <div className="text-[10px] text-on-surface/60 bg-white/5 p-2 rounded">
                   Monochrome sensor — full photon flux, native IR sensitivity.
                 </div>
              </div>
           </DeviceCard>

           <div className="flex flex-col gap-6">
              <DeviceCard deviceId="c925e-video" title="C925e Video (Secondary)" accent="rx">
                 <div className="flex flex-col gap-4">
                    <MdSelect
                      label="Mode"
                      value={settings.devices['c925e-video']?.mode || 'both'}
                      onSelected={(v) => updateDevice('c925e-video', { mode: v })}
                    >
                      <MdSelectOption value="visual_mic">Visual Microphone</MdSelectOption>
                      <MdSelectOption value="scene_ref">Scene Reference</MdSelectOption>
                      <MdSelectOption value="both">Both</MdSelectOption>
                    </MdSelect>
                    <div className="text-[10px] text-br-tan font-bold uppercase italic">
                      Rolling shutter — not suitable for stereo depth.
                    </div>
                 </div>
              </DeviceCard>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                 <DeviceCard deviceId="ir-emitter-array" title="IR Emitter / Receiver" accent="rx">
                    <div className="flex flex-col gap-4 opacity-50 grayscale">
                       <MdSelect label="Pattern" disabled>
                          <MdSelectOption value="structured">Structured Light</MdSelectOption>
                       </MdSelect>
                       <div className="text-[10px] font-mono uppercase">Driver: Pico 2 PIO</div>
                    </div>
                 </DeviceCard>
                 <DeviceCard deviceId="mems-microphones" title="MEMS Microphones" accent="rx">
                    <div className="flex flex-col gap-4 opacity-50 grayscale">
                       <MdSelect label="PDM Clock" disabled>
                          <MdSelectOption value="3.072">3.072 MHz</MdSelectOption>
                       </MdSelect>
                       <span className="text-[10px] uppercase font-bold text-on-surface/50">Array Geometry Pending</span>
                    </div>
                 </DeviceCard>
              </div>
           </div>
        </div>
      </section>

      <md-divider></md-divider>

      <section>
        <div className="flex items-center gap-4 mb-6">
          <h2 className="text-xl font-bold tracking-tight uppercase">Master Clock and UWB</h2>
          <div className="flex-1 h-px bg-white/5"></div>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
           <DeviceCard deviceId="pico-2" title="Pico 2 (RP2350)" accent="config">
              <div className="flex flex-col gap-5">
                 <div className="p-3 rounded bg-br-tan/10 border border-br-tan/20 flex flex-col gap-1">
                   <span className="text-xs font-bold text-br-tan uppercase">Master Clock</span>
                   <span className="text-[10px] text-on-surface/70">ALL SENSOR TIMESTAMPS SLAVED TO PPS.</span>
                 </div>

                 <div className="flex justify-between items-center text-xs">
                    <span className="text-on-surface/50 uppercase">Clock Frequency</span>
                    <span className="font-mono">150 MHz (6.67 ns resolution)</span>
                 </div>

                 <MdSelect
                   label="PPS Output GPIO"
                   value={String(picoSettings.pps_pin)}
                   onSelected={(v) => updateDevice('pico-2', { pps_pin: parseInt(v) })}
                 >
                   {[0, 1, 2, 3, 4, 5].map(p => (
                     <MdSelectOption key={p} value={String(p)}>GPIO {p}</MdSelectOption>
                   ))}
                 </MdSelect>

                 <MdTextField
                    label="Serial Port"
                    value={picoSettings.serial_port}
                    onInput={(v) => updateDevice('pico-2', { serial_port: v })}
                 />

                 <div className="flex flex-col gap-4 pt-2 border-t border-white/5 mt-2">
                    <h4 className="text-[10px] font-bold text-on-surface/50 uppercase tracking-widest">UWB Impulse Radio</h4>
                    <MdSelect
                      label="Mode"
                      value={picoSettings.uwb_mode}
                      onSelected={(v) => updateDevice('pico-2', { uwb_mode: v })}
                    >
                      <MdSelectOption value="disabled">Disabled</MdSelectOption>
                      <MdSelectOption value="tx">TX Mode</MdSelectOption>
                      <MdSelectOption value="rx">RX Mode</MdSelectOption>
                      <MdSelectOption value="ranging">Ranging</MdSelectOption>
                    </MdSelect>

                    <div className="grid grid-cols-2 gap-3 text-[10px] font-mono">
                       <div className="flex flex-col">
                          <span className="text-on-surface/40 uppercase">Pulse Width</span>
                          <span>~6.67 ns</span>
                       </div>
                       <div className="flex flex-col">
                          <span className="text-on-surface/40 uppercase">Effective BW</span>
                          <span>DC – 150 MHz</span>
                       </div>
                    </div>
                 </div>
              </div>
           </DeviceCard>

           <div className="flex flex-col gap-6">
              <md-elevated-card style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}>
                <div className="p-5">
                   <h4 className="text-[10px] font-bold text-on-surface/50 uppercase tracking-widest mb-4">Modality Reference</h4>
                   <div className="overflow-x-auto">
                      <table className="w-full text-left text-[10px]">
                         <thead className="border-b border-white/10 text-on-surface/40 uppercase">
                            <tr>
                               <th className="pb-2 font-medium">Modality</th>
                               <th className="pb-2 font-medium">Hardware</th>
                               <th className="pb-2 font-medium">Dir</th>
                               <th className="pb-2 font-medium">Band</th>
                            </tr>
                         </thead>
                         <tbody className="text-on-surface/80">
                            <tr className="border-b border-white/5">
                               <td className="py-2">Continuous Wave RF</td>
                               <td className="py-2">PlutoSDR+ PA</td>
                               <td className="py-2"><span className="px-1.5 py-0.5 rounded-full bg-tx/20 text-tx font-bold">TX</span><span className="px-1.5 py-0.5 rounded-full bg-rx/20 text-rx font-bold ml-1">RX</span></td>
                               <td className="py-2 font-mono">70M-6G</td>
                            </tr>
                            <tr className="border-b border-white/5">
                               <td className="py-2">Passive RF Scan</td>
                               <td className="py-2">RTL-SDR + Youloop</td>
                               <td className="py-2"><span className="px-1.5 py-0.5 rounded-full bg-rx/20 text-rx font-bold">RX</span></td>
                               <td className="py-2 font-mono">10k-300M</td>
                            </tr>
                            <tr className="border-b border-white/5">
                               <td className="py-2">UWB Impulse Radio</td>
                               <td className="py-2">Pico 2 PIO</td>
                               <td className="py-2"><span className="px-1.5 py-0.5 rounded-full bg-tx/20 text-tx font-bold">TX</span><span className="px-1.5 py-0.5 rounded-full bg-rx/20 text-rx font-bold ml-1">RX</span></td>
                               <td className="py-2 font-mono">DC-150M</td>
                            </tr>
                            <tr>
                               <td className="py-3" colSpan={4}>
                                  <div className="text-[9px] text-on-surface/40 leading-relaxed">
                                    Bistatic MIMO, TDOA, Bearing via null axis,
                                    Raw acoustic (no preprocessing), Visual mic ~24kHz,
                                    Differential magnetic (1Hz-192kHz).
                                  </div>
                               </td>
                            </tr>
                         </tbody>
                      </table>
                   </div>
                </div>
              </md-elevated-card>
           </div>
        </div>
      </section>
    </div>
  );
}
