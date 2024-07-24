//
//  ArgonSendSheet.swift
//  LocalchainIOS
//
//  Created by Blake Byrnes on 4/2/24.
//

import BigNumber
import Foundation
import QRCode
import SwiftUI

struct ArgonSendSheet: View {
  enum FocusedField {
    case argons
  }

  @Binding var isPresented: Bool
  @State var argons: Double?
  @EnvironmentObject var localchain: LocalchainBridge
  @State var errorMessage: String?
  @State var argonFile: ArgonFileTransfer?
  @State private var isShareSheetPresented = false
  @State private var itemsToShare: [Any] = []

  var body: some View {
    NavigationView {
      GeometryReader { geo in
        VStack {
          HStack {
            Spacer()
            Button(action: { isPresented = false }) {
              Image(systemName: "xmark.circle")
                .resizable()
                .tint(.gray)
                .frame(width: 25, height: 25)
            }
          }

          Text("Send Money")
            .fontWeight(.bold)
            .font(.title)
            .foregroundColor(.accentColor)
            .padding(.bottom, 10)

          if let argonFile = argonFile, !isShareSheetPresented {
            QRCodeDocumentUIView(document: getQr(json: argonFile.json))
              .frame(width: geo.size.width * 0.85, height: geo.size.width * 0.85, alignment: .center)

            Text("\(argonFile.name)")
              .font(.footnote)

          } else {
            if errorMessage != nil {
              Text("\(errorMessage ?? "An error occurred")")
                .font(.footnote)
                .fontWeight(.thin)
                .foregroundColor(.red)
            } else {
              Text("Enter the amount to send.")
                .font(.footnote)
                .fontWeight(.thin)
                .foregroundStyle(.black)
            }
            CurrencyTextField(
              "₳0.00",
              value: $argons,
              isResponder: $isPresented,
              alwaysShowFractions: false,
              numberOfDecimalPlaces: 2,
              currencySymbol: "₳"
            )
            .font(.largeTitle)
            .multilineTextAlignment(TextAlignment.center)
            .frame(height: 50)
            .padding()
            .textFieldStyle(.roundedBorder)
            .border(.secondary)
            .padding(.bottom, 20)

            Button {
              Task {
                let file = await createArgonFile()
                await MainActor.run {
                  argonFile = file
                }
              }
            } label: {
              Label("Transfer Physically", image: "send")
                .frame(maxWidth: .infinity)
                .fontWeight(.bold)
                .font(.headline)
                .padding(.vertical, 10)
                .tint(.accentColor)
            }
            .background(.white)
            .cornerRadius(8)
            .overlay(
              RoundedRectangle(cornerRadius: 8)
                .stroke(Color.black.opacity(0.8), lineWidth: 0.6)
            )
            .foregroundColor(.accentColor)

            Text("OR")
              .foregroundColor(.gray)

            Button {
              Task {
                let file = await createArgonFile()
                await MainActor.run {
                  if let file = file {
                    isShareSheetPresented = true
                    itemsToShare = [ArgonFileItemSource(file)]
                  }
                }
              }

            } label: {
              Label("Send Using...", systemImage: "paperplane")
                .frame(maxWidth: .infinity)
                .fontWeight(.bold)
                .font(.headline)
                .padding(.vertical, 10)
            }
            .background(Color.accentColor)
            .cornerRadius(8)
            .overlay(
              RoundedRectangle(cornerRadius: 8)
                .stroke(Color.accentColor, lineWidth: 0.6)
            )
            .foregroundColor(.white)
          }
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, 20)
        .padding(.vertical, 20)
      }
    }
    .sheet(isPresented: $isShareSheetPresented) {
      ShareSheet(items: itemsToShare)
        .preferredColorScheme(.light)
        .ignoresSafeArea()
    }
    .onDisappear {
      errorMessage = nil
    }
  }

  struct ShareSheet: UIViewControllerRepresentable {
    var items: [Any]

    func makeUIViewController(context _: Context) -> UIActivityViewController {
      let controller = UIActivityViewController(activityItems: items, applicationActivities: nil)

      return controller
    }

    func updateUIViewController(_: UIActivityViewController, context _: Context) {}
  }

  func createArgonFile() async -> ArgonFileTransfer? {
    guard let argons = argons else {
      errorMessage = "Please input the amount"
      return nil
    }
    let milligons = UInt64(argons * 1_000)
    let localchain = localchain
    do {
      let file = try await localchain.createArgonFile(
        isRequesting: false,
        milligons: milligons
      )

      return file
    } catch let AppError.insufficientBalance(balance) {
      await MainActor.run {
        errorMessage = "You only have \(balance)"
      }
    } catch let UniffiError.Generic(message) {
      await MainActor.run {
        errorMessage = message
      }
    } catch {
      await MainActor.run {
        errorMessage = "\(error)"
      }
    }
    return nil
  }
}

func getQr(json: String) -> QRCode.Document {
  let doc = QRCode.Document()

  doc.errorCorrection = .default
  doc.design.backgroundColor(UIColor.clear.cgColor)
  doc.design.shape.eye = QRCode.EyeShape.Squircle()
  doc.design.style.eye = QRCode.FillStyle.Solid(UIColor.black.cgColor)
  doc.design.shape.onPixels = QRCode.PixelShape.Circle()
//    doc.data = Data(bytes)

  doc.utf8String = json

  return doc
}

struct ContentView_Previews: PreviewProvider {
  static var previews: some View {
    ContentView()
  }
}
