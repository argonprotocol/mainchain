import SwiftUI
import UIKit

struct ShareSheet: UIViewControllerRepresentable {
  var items: [Any]

  func makeUIViewController(context: Context) -> UIViewController {
    let controller = UIViewController()
    DispatchQueue.main.async {
      let activityViewController = UIActivityViewController(activityItems: items, applicationActivities: nil)
      context.coordinator.parent = controller
      controller.present(activityViewController, animated: true, completion: nil)
    }
    return controller
  }

  func updateUIViewController(_: UIViewController, context _: Context) {}

  func makeCoordinator() -> Coordinator {
    Coordinator()
  }

  class Coordinator: NSObject {
    var parent: UIViewController?
  }
}

extension View {
  func shareSheet(isPresented: Binding<Bool>, items: [Any]) -> some View {
    background(
      ShareSheetWrapper(isPresented: isPresented, items: items)
    )
  }
}

struct ShareSheetWrapper: UIViewControllerRepresentable {
  @Binding var isPresented: Bool
  var items: [Any]

  func makeUIViewController(context: Context) -> UIViewController {
    let controller = UIViewController()
    context.coordinator.parent = controller
    return controller
  }

  func updateUIViewController(_ uiViewController: UIViewController, context: Context) {
    if isPresented && context.coordinator.parent?.presentedViewController == nil {
      let activityViewController = UIActivityViewController(activityItems: items, applicationActivities: nil)
      activityViewController.completionWithItemsHandler = { _, _, _, _ in
        self.isPresented = false
      }
      uiViewController.present(activityViewController, animated: true, completion: nil)
    }
  }

  func makeCoordinator() -> Coordinator {
    Coordinator(isPresented: $isPresented)
  }

  class Coordinator: NSObject {
    @Binding var isPresented: Bool
    var parent: UIViewController?

    init(isPresented: Binding<Bool>) {
      _isPresented = isPresented
    }
  }
}
