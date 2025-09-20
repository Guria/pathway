import Cocoa
import os

private let subsystem = "dev.pathway.router"

final class PathwayAppDelegate: NSObject, NSApplicationDelegate {
    private let logger = Logger(subsystem: subsystem, category: "shim")
    private let eventManager = NSAppleEventManager.shared()
    private let syncQueue = DispatchQueue(label: "dev.pathway.router.shim.queue")
    private var pendingLaunches = 0
    private var activeProcesses: [Process] = []
    private var terminationWorkItem: DispatchWorkItem?
    private let terminationDelay: TimeInterval = 1.0

    func applicationDidFinishLaunching(_ notification: Notification) {
        eventManager.setEventHandler(self,
                                     andSelector: #selector(handleGetURLEvent(event:replyEvent:)),
                                     forEventClass: AEEventClass(kInternetEventClass),
                                     andEventID: AEEventID(kAEGetURL))
        scheduleTerminationCheck()
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        handle(urls: urls)
    }

    @objc private func handleGetURLEvent(event: NSAppleEventDescriptor, replyEvent: NSAppleEventDescriptor) {
        var urls: [URL] = []
        if let directObject = event.paramDescriptor(forKeyword: keyDirectObject) {
            if directObject.descriptorType == typeAEList {
                for index in 1...directObject.numberOfItems {
                    if let value = directObject.atIndex(index)?.stringValue, let url = URL(string: value) {
                        urls.append(url)
                    }
                }
            } else if let value = directObject.stringValue, let url = URL(string: value) {
                urls.append(url)
            }
        }
        handle(urls: urls)
    }

    private func handle(urls: [URL]) {
        syncQueue.async { [weak self] in
            guard let self = self else { return }
            guard !urls.isEmpty else {
                self.logger.debug("Received empty URL payload")
                self.scheduleTerminationCheckLocked()
                return
            }

            let candidateURL = Bundle.main.url(forAuxiliaryExecutable: "pathway")
                ?? Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/pathway")
            guard FileManager.default.isExecutableFile(atPath: candidateURL.path) else {
                self.logger.fault("Unable to locate bundled pathway binary at \(candidateURL.path, privacy: .public)")
                self.scheduleTerminationCheckLocked()
                return
            }
            let pathwayURL = candidateURL

            self.pendingLaunches += 1
            let process = Process()
            process.executableURL = pathwayURL
            process.arguments = urls.map { $0.absoluteString }
            self.activeProcesses.append(process)

            var environment = ProcessInfo.processInfo.environment
            environment["PATHWAY_LAUNCHED_FROM_BUNDLE"] = "1"
            if let identifier = Bundle.main.bundleIdentifier {
                environment["PATHWAY_BUNDLE_IDENTIFIER"] = identifier
            }
            process.environment = environment

            process.standardInput = FileHandle.nullDevice
            process.standardOutput = FileHandle.nullDevice
            process.standardError = FileHandle.nullDevice

            process.terminationHandler = { [weak self] _ in
                guard let self = self else { return }
                self.syncQueue.async {
                    self.activeProcesses.removeAll { $0 === process }
                    self.pendingLaunches = max(0, self.pendingLaunches - 1)
                    self.scheduleTerminationCheckLocked()
                }
            }

            do {
                try process.run()
                self.logger.info("Forwarded \(urls.count) URL(s) to pathway binary")
            } catch {
                self.logger.error("Failed to launch pathway binary: \(error.localizedDescription)")
                self.activeProcesses.removeAll { $0 === process }
                self.pendingLaunches = max(0, self.pendingLaunches - 1)
                self.scheduleTerminationCheckLocked()
            }
        }
    }

    private func scheduleTerminationCheck() {
        syncQueue.async { [weak self] in
            self?.scheduleTerminationCheckLocked()
        }
    }

    private func scheduleTerminationCheckLocked() {
        terminationWorkItem?.cancel()
        let workItem = DispatchWorkItem { [weak self] in
            guard let self = self else { return }
            self.syncQueue.async {
                if self.pendingLaunches == 0 {
                    self.logger.debug("No pending launches, terminating shim")
                    DispatchQueue.main.async {
                        NSApp.terminate(nil)
                    }
                }
            }
        }
        terminationWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + terminationDelay, execute: workItem)
    }
}

@main
final class PathwayShim {
    static func main() {
        let application = NSApplication.shared
        application.setActivationPolicy(.prohibited)
        let delegate = PathwayAppDelegate()
        application.delegate = delegate
        application.run()
    }
}
