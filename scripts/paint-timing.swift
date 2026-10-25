// Times how long WebKit takes to parse, lay out and paint each SVG given on
// the command line, the way the app's sheet shows a new preview. macOS only.
//
// The core's render time is only part of an edit: on a large site WebKit
// spends several times longer painting the SVG than the core took to write
// it. Each file is shown a few times and the median printed; an empty sheet
// takes about 33 ms, two frames at 60 Hz, so read the numbers above that.
//
//   swiftc -O scripts/paint-timing.swift -o target/paint-timing
//   target/paint-timing .scratch/svgs/*.svg

import Cocoa
import WebKit

// Roughly the sheet's size on a laptop screen.
let viewWidth = 1400
let viewHeight = 1000
let runs = 5

let files = Array(CommandLine.arguments.dropFirst())
if files.isEmpty {
    print("usage: paint-timing <file.svg>...")
    exit(1)
}

let app = NSApplication.shared
let window = NSWindow(
    contentRect: NSRect(x: 0, y: 0, width: viewWidth, height: viewHeight),
    styleMask: [.titled], backing: .buffered, defer: false)
let web = WKWebView(frame: window.contentView!.bounds)
window.contentView!.addSubview(web)
window.makeKeyAndOrderFront(nil)

// Shows the SVG `runs` times, each after two frames of an empty sheet, and
// returns the median time from setting it until two frames later.
let timing = """
const box = document.getElementById('box');
const nextFrames = () => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
const times = [];
for (let run = 0; run < runs; run++) {
  box.innerHTML = '';
  await nextFrames();
  const start = performance.now();
  box.innerHTML = svg;
  box.firstElementChild.getBoundingClientRect();
  await nextFrames();
  times.push(performance.now() - start);
}
times.sort((a, b) => a - b);
return times[Math.floor(times.length / 2)];
"""

class Driver: NSObject, WKNavigationDelegate {
    var next = 0

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        timeNext()
    }

    func timeNext() {
        guard next < files.count else { exit(0) }
        let path = files[next]
        next += 1
        guard let svg = try? String(contentsOfFile: path, encoding: .utf8) else {
            print("cannot read \(path)")
            timeNext()
            return
        }
        web.callAsyncJavaScript(timing, arguments: ["svg": svg, "runs": runs], in: nil, in: .page) { result in
            let name = (path as NSString).lastPathComponent
            switch result {
            case .success(let value):
                print(String(format: "%8.1f ms  %@", (value as? Double) ?? -1, name))
            case .failure(let error):
                print("\(name): \(error.localizedDescription)")
            }
            self.timeNext()
        }
    }
}

let driver = Driver()
web.navigationDelegate = driver
web.loadHTMLString(
    "<html><body style='margin:0'><div id='box' style='width:\(viewWidth)px;height:\(viewHeight)px'></div></body></html>",
    baseURL: nil)
app.run()
