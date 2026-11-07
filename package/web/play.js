const PROTOCOL = "rackforge.plugin.web@1";
const DIAL_SWEEP = 270;
const status = document.getElementById("status");
const pending = new Map();
let sequence = 0;
let contextKey = null;
let bootstrap = 0;
let ready = false;

function request(method, params = {}) {
  const request_id = `rf-epiano-${++sequence}`;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(request_id);
      reject(new Error("RackForge did not respond"));
    }, 5000);
    pending.set(request_id, { resolve, reject, timer });
    parent.postMessage({ protocol: PROTOCOL, kind: "request", request_id, method, params }, "*");
  });
}

function setStatus(text, error = false) {
  status.textContent = text;
  status.dataset.error = String(error);
  status.dataset.ready = String(ready && !error);
}

function display(control, value) {
  control.input.value = value;
  control.dial.style.setProperty("--angle", `${(value - 0.5) * DIAL_SWEEP}deg`);
  const reading = `${Math.round(value * 100)}%`;
  control.output.value = reading;
  control.input.setAttribute("aria-valuetext", reading);
}

// The native range retains its keyboard and accessibility behavior.
function bindDialPointer(input) {
  let gesture;
  const update = (value) => {
    const stepped = Math.round(Math.min(1, Math.max(0, value)) * 1000) / 1000;
    if (input.valueAsNumber === stepped) return;
    input.valueAsNumber = stepped;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  };

  input.addEventListener("pointerdown", (event) => {
    if (!event.isPrimary || event.button !== 0 || input.disabled
        || matchMedia("(forced-colors: active)").matches) return;
    event.preventDefault();
    input.focus({ preventScroll: true });
    input.setPointerCapture(event.pointerId);
    gesture = { id: event.pointerId, x: event.clientX, y: event.clientY,
      start: input.valueAsNumber, axis: null };
  });
  input.addEventListener("pointermove", (event) => {
    if (gesture?.id !== event.pointerId || input.disabled) return;
    const dx = event.clientX - gesture.x;
    const dy = gesture.y - event.clientY;
    if (!gesture.axis && Math.hypot(dx, dy) >= 4) {
      gesture.axis = Math.abs(dx) > Math.abs(dy) ? "x" : "y";
    }
    if (gesture.axis) update(gesture.start + (gesture.axis === "x" ? dx : dy) / 140);
  });
  input.addEventListener("pointerup", (event) => {
    if (gesture?.id !== event.pointerId || gesture.axis || input.disabled) return;
    const rect = input.getBoundingClientRect();
    const x = event.clientX - (rect.left + rect.width / 2);
    const y = event.clientY - (rect.top + rect.height / 2);
    if (Math.hypot(x, y) >= rect.width * 0.19) {
      const degrees = Math.atan2(x, -y) * 180 / Math.PI;
      update(0.5 + degrees / DIAL_SWEEP);
    }
  });
  input.addEventListener("lostpointercapture", (event) => {
    if (gesture?.id === event.pointerId) gesture = undefined;
  });
}

const controls = ["volume", "tremolo", "speaker"].map((id, index) => {
  const input = document.getElementById(id);
  const output = document.getElementById(`${id}-value`);
  const dial = input.parentElement;
  const control = { input, output, dial, index, canonical: input.valueAsNumber, next: null, sending: false };
  bindDialPointer(input);
  display(control, control.canonical);
  input.addEventListener("input", () => {
    const value = input.valueAsNumber;
    if (!ready || !Number.isFinite(value)) return;
    display(control, value);
    control.next = value;
    void send(control);
  });
  return control;
});

async function send(control) {
  if (control.sending) return;
  control.sending = true;
  const generation = bootstrap;
  try {
    // Coalesce pointer motion while a previous write is in flight.
    while (control.next !== null && generation === bootstrap) {
      const value = control.next;
      control.next = null;
      const result = await request("plugin.set_parameter", { parameter_index: control.index, value });
      if (generation === bootstrap) {
        control.canonical = Number.isFinite(result?.value) && result.value >= 0 && result.value <= 1
          ? result.value : value;
        if (control.next === null) display(control, control.canonical);
        setStatus("Ready");
      }
    }
  } catch (error) {
    if (generation === bootstrap) {
      control.next = null;
      display(control, control.canonical);
      setStatus(error.message, true);
    }
  } finally {
    control.sending = false;
    if (ready && control.next !== null) void send(control);
  }
}

async function loadControls() {
  const generation = ++bootstrap;
  ready = false;
  for (const control of controls) {
    control.next = null;
    control.input.disabled = true;
  }
  try {
    const result = await request("plugin.parameters");
    if (generation !== bootstrap) return;
    const values = new Map((result?.values ?? []).map(({ index, value }) => [index, value]));
    for (const control of controls) {
      const value = values.get(control.index);
      if (!Number.isFinite(value) || value < 0 || value > 1) throw new Error("Sound controls unavailable");
      control.canonical = value;
      display(control, value);
    }
    ready = true;
    for (const control of controls) control.input.disabled = false;
    setStatus("Ready");
  } catch (error) {
    if (generation === bootstrap) setStatus(error.message, true);
  }
}

window.addEventListener("message", ({ source, data: message }) => {
  if (source !== parent || message?.protocol !== PROTOCOL) return;
  if (message.kind === "response") {
    const entry = pending.get(message.request_id);
    if (!entry) return;
    pending.delete(message.request_id);
    clearTimeout(entry.timer);
    if (message.ok) entry.resolve(message.result);
    else entry.reject(new Error(typeof message.error === "string" ? message.error : "RackForge refused the change"));
    return;
  }
  if (message.kind === "context") {
    document.documentElement.dataset.lighting = message.host?.lighting === "day" ? "day" : "stage";
    const key = JSON.stringify([message.instance?.instance_id, message.instance?.selected_sound_id, message.isolated]);
    if (key !== contextKey) {
      contextKey = key;
      void loadControls();
    }
    return;
  }
  if (message.kind === "parameter_changed") {
    if (!Number.isInteger(message.parameter_index)) return;
    const control = controls[message.parameter_index];
    const value = message.value;
    if (!control || !Number.isFinite(value) || value < 0 || value > 1) return;
    control.canonical = value;
    if (ready && !control.sending && control.next === null) display(control, value);
  }
});

document.querySelector("rf-program-select").addEventListener("rf-program-error", (event) => {
  setStatus(event.detail.error, true);
});

parent.postMessage({ protocol: PROTOCOL, kind: "ready" }, "*");
