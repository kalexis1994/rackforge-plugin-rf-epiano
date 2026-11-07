"""Exercise the packaged PLAY surface in a sandboxed iframe (requires Playwright)."""
import argparse
import json
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = b'''<!doctype html><meta charset="utf-8"><title>RF-EPiano panel verification</title>
<style>body{margin:0}iframe{width:100%;height:100vh;border:0}</style>
<iframe title="RF-EPiano" sandbox="allow-scripts allow-same-origin"></iframe>
<script>
const frame = document.querySelector('iframe');
const protocol = 'rackforge.plugin.web@1';
window.values = [0.72, 0.35, 0.2];
window.writes = [];
window.rejectWrite = false;
window.rejectSelect = false;
window.selections = [];
const catalog = __CATALOG__;
const sounds = catalog.presets.map(({id, name, bank, description, category}) => ({
  id, name, bank, detail: description ?? category
}));
const defaults = {
  default: [.8, .5, 0], direct: [.8, 0, 0], cabinet: [.8, 0, 1],
  tremolo: [.8, .65, 1], bark: [.7, 0, 0]
};
let lightingMode = 'stage';
let selectedSound = 'default';
window.send = (data) => frame.contentWindow.postMessage({ protocol, ...data }, '*');
window.context = (lighting = lightingMode, selected_sound_id = selectedSound) => {
  lightingMode = lighting;
  selectedSound = selected_sound_id;
  send({
    kind: 'context',
    instance: { instance_id: 'epiano-fixture', selected_sound_id, sounds, banks: catalog.banks },
    host: { lighting }, isolated: false
  });
};
addEventListener('message', ({source, data}) => {
  if (source !== frame.contentWindow || data?.protocol !== protocol) return;
  if (data.kind === 'ready') { context(); return; }
  if (data.kind !== 'request') return;
  const reply = (ok, result, error) => send({kind:'response', request_id:data.request_id, ok, result, error});
  if (data.method === 'plugin.parameters') {
    reply(true, {values: values.map((value, index) => ({index, value}))});
  } else if (data.method === 'plugin.set_parameter') {
    if (rejectWrite) { reply(false, null, 'Control unavailable'); return; }
    const {parameter_index, value} = data.params;
    writes.push([parameter_index, value]);
    values[parameter_index] = value;
    reply(true, {value});
    send({kind:'parameter_changed', parameter_index, value});
  } else if (data.method === 'plugin.select_sound') {
    const {sound_id} = data.params;
    if (rejectSelect || !sounds.some(sound => sound.id === sound_id)) {
      reply(false, null, 'Program unavailable');
      return;
    }
    selections.push(sound_id);
    values = [...defaults[sound_id]];
    reply(true, {sound_id});
    context(lightingMode, sound_id);
  }
});
frame.addEventListener('load', () => {
  const script = frame.contentDocument.createElement('script');
  script.type = 'module';
  script.src = '/rackforge-plugin-kit/program-select.js';
  frame.contentDocument.head.append(script);
  context();
}, {once: true});
frame.src = '/web/play.html';
</script>'''


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, fixture, kit, **kwargs):
        self.fixture = fixture
        self.kit = kit
        super().__init__(*args, **kwargs)

    def do_GET(self):
        if self.path == "/favicon.ico":
            self.send_response(204)
            self.end_headers()
        elif self.path == "/":
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(self.fixture)
        elif self.path == "/rackforge-plugin-kit/program-select.js":
            self.send_response(200)
            self.send_header("Content-Type", "text/javascript; charset=utf-8")
            self.end_headers()
            self.wfile.write(self.kit)
        else:
            super().do_GET()

    def end_headers(self):
        if self.path == "/web/play.html":
            self.send_header("Content-Security-Policy", "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; frame-ancestors 'self'; connect-src 'none'")
        super().end_headers()

    def log_message(self, *_):
        pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", type=Path, default=ROOT / "package")
    parser.add_argument("--browser", type=Path, help="Use a system Chromium executable")
    parser.add_argument("--kit", type=Path,
                        default=ROOT.parent / "rackforge/web/dist/rackforge-plugin-kit/program-select.js",
                        help="Path to the built RackForge program selector")
    parser.add_argument("--screenshots", type=Path, default=ROOT / "work" / "panel-check")
    args = parser.parse_args()
    if not args.kit.is_file():
        parser.error("Build RackForge's web UI or pass --kit with its compiled program-select.js")
    catalog = json.loads((args.package / "metadata/presets.json").read_text(encoding="utf-8"))
    fixture = FIXTURE.replace(b"__CATALOG__", json.dumps(catalog).replace("<", "\\u003c").encode())
    handler = partial(Handler, directory=str(args.package), fixture=fixture, kit=args.kit.read_bytes())
    args.screenshots.mkdir(parents=True, exist_ok=True)
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        Thread(target=server.serve_forever, daemon=True).start()
        try:
            with sync_playwright() as playwright:
                browser = playwright.chromium.launch(executable_path=args.browser)
                page = browser.new_page(viewport={"width": 1140, "height": 540})
                errors = []
                page.on("pageerror", lambda error: errors.append(str(error)))
                page.on("console", lambda message: errors.append(message.text)
                        if message.type == "error" else None)
                page.goto(f"http://127.0.0.1:{server.server_port}/")
                panel = page.frame_locator("iframe")
                panel.get_by_text("Ready", exact=True).wait_for()
                inner = page.frame(url=lambda url: url.endswith("/web/play.html"))
                assert inner.evaluate("""async () => {
                  await document.fonts.ready;
                  return [...document.fonts].length === 2
                    && [...document.fonts].every(font => font.status === 'loaded');
                }""")
                sliders = [panel.get_by_role("slider", name=name) for name in ("Volume", "Tremolo", "Speaker")]
                assert [s.input_value() for s in sliders] == ["0.72", "0.35", "0.2"]
                for index, slider in enumerate(sliders):
                    slider.press("ArrowRight")
                    page.wait_for_function(f"writes.some(([index, value]) => index === {index})")
                assert page.evaluate("writes.map(([index]) => index)") == [0, 1, 2]

                page.evaluate("send({kind:'parameter_changed',parameter_index:1,value:.68})")
                panel.get_by_text("68%", exact=True).wait_for()
                assert sliders[1].input_value() == "0.68"
                inner.evaluate("document.activeElement.blur()")
                page.screenshot(path=str(args.screenshots / "stage-desktop.png"))

                # Drag relative to the current value, then tap the top of the dial.
                dial = sliders[2].bounding_box()
                x, y = dial["x"] + dial["width"] / 2, dial["y"] + dial["height"] / 2
                page.mouse.move(x, y)
                page.mouse.down()
                page.mouse.move(x, y - 16, steps=4)
                page.mouse.move(x - 55, y - 42, steps=5)
                page.mouse.up()
                page.wait_for_function("Math.abs(values[2] - .501) < .002")
                page.mouse.click(x, dial["y"] + 4)
                page.wait_for_function("Math.abs(values[2] - .5) < .002")
                assert inner.evaluate("document.querySelector('#speaker').parentElement.style.getPropertyValue('--angle')") == "0deg"

                page.evaluate("rejectWrite = true")
                sliders[0].press("ArrowRight")
                panel.get_by_text("Control unavailable", exact=True).wait_for()
                assert sliders[0].input_value() == "0.721"

                selector = panel.locator("rf-program-select")
                def program(name):
                    return selector.get_by_role("button", name=f"Program: {name}. Choose another", exact=True)

                program("200A Original").wait_for()
                selector.get_by_role("button", name="Previous program").click()
                program("200A Bark").wait_for()
                panel.get_by_text("70%", exact=True).wait_for()
                assert [s.input_value() for s in sliders] == ["0.7", "0", "0"]
                assert page.evaluate("selections") == ["bark"]

                inner.evaluate("document.activeElement.blur()")
                page.screenshot(path=str(args.screenshots / "stage-desktop.png"))

                program("200A Bark").click()
                dialog = selector.get_by_role("dialog", name="Programs")
                assert dialog.get_by_role("option").count() == 5
                page.screenshot(path=str(args.screenshots / "programs-desktop.png"))
                search = dialog.get_by_role("combobox")
                search.fill("65%")
                assert dialog.get_by_role("option").count() == 1
                search.press("Enter")
                program("200A Cabinet Tremolo").wait_for()
                panel.get_by_text("65%", exact=True).wait_for()
                assert [s.input_value() for s in sliders] == ["0.8", "0.65", "1"]

                page.evaluate("rejectSelect = true")
                selector.get_by_role("button", name="Next program").click()
                panel.get_by_text("Program unavailable", exact=True).wait_for()
                program("200A Cabinet Tremolo").wait_for()
                assert [s.input_value() for s in sliders] == ["0.8", "0.65", "1"]
                assert page.evaluate("selections") == ["bark", "tremolo"]

                page.set_viewport_size({"width": 390, "height": 680})
                page.evaluate("values=[.8,.5,0]; rejectWrite=false; context('day','default')")
                panel.get_by_text("Ready", exact=True).wait_for()
                assert [s.input_value() for s in sliders] == ["0.8", "0.5", "0"]
                program("200A Original").wait_for()
                assert inner.evaluate("document.documentElement.dataset.lighting") == "day"
                assert inner.evaluate("document.documentElement.scrollWidth <= innerWidth")
                inner.evaluate("document.activeElement.blur()")
                page.screenshot(path=str(args.screenshots / "day-mobile.png"))
                page.set_viewport_size({"width": 320, "height": 420})
                assert inner.evaluate("document.documentElement.scrollWidth <= innerWidth")
                assert not errors, errors
                browser.close()
        finally:
            server.shutdown()
    print("RF-EPIANO_PANEL_OK desktop=stage mobile=day controls=3 program_kit=host")


if __name__ == "__main__":
    main()
