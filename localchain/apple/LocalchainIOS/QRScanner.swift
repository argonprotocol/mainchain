
import Foundation
import QRCodeDetector
import SwiftUI

struct QRScanner: UIViewControllerRepresentable {
  typealias UIViewControllerType = UIViewController

  @Binding var isPresented: Bool
  let callback: (_ message: String) -> Void

  let detector = QRCodeDetector.VideoDetector()

  func makeUIViewController(context _: Context) -> UIViewController {
    let controller = UIViewController()
    controller.modalPresentationStyle = .fullScreen

    do {
      try detector.startDetecting { _, features in
        features.forEach { feature in
          if let message = feature.messageString, let file = try? JSONDecoder().decode(
            ArgonFile.self,
            from: Data(message.utf8)
          ) {
            detector.stopDetection()
            isPresented = false
            self.callback(message)
          }
        }
      }
    } catch {}
    let view = controller.view!
    let pl = try! detector.makePreviewLayer()

    view.layer.addSublayer(pl)
    pl.frame = view.layer.bounds
    return controller
  }

  func updateUIViewController(_: UIViewController, context _: Context) {
    if isPresented == false {
      detector.stopDetection()
    }
  }
}
