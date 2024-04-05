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
  @State var argonRequestType = "Send"
  @State var argons: Double?
  @FocusState var focusField: FocusedField?
  @EnvironmentObject var localchain: LocalchainBridge
  @State var errorMessage: String?
  @State var argonFile: ArgonFileTransfer?

  var body: some View {
    NavigationView {
      GeometryReader { geo in
        VStack {
          if let argonFile = argonFile {
            QRCodeDocumentUIView(document: getQr(json: argonFile.json))
              .frame(width: geo.size.width * 0.85, height: geo.size.width * 0.85, alignment: .center)

            Text("\(argonFile.name)")
              .font(.footnote)

            ShareLink(
              item: argonFile,
              preview: SharePreview(
                argonFile.name,
                icon: Image("argfile")
              )
            ) {
              Label("Send File", systemImage: "square.and.arrow.up")
            }
          } else {
            Picker("Do you want to send or receive funds?", selection: $argonRequestType) {
              Text("Send Money").tag("Send")
              Text("Ask for Money").tag("Request")
            }
            .pickerStyle(.segmented)

            if errorMessage != nil {
              Text("\(errorMessage ?? "An error occurred")")
                .foregroundColor(.red)
            }
            CurrencyTextField(
              "₳0.00",
              value: $argons,
              alwaysShowFractions: false,
              numberOfDecimalPlaces: 2,
              currencySymbol: "₳"
            )
            .font(.largeTitle)
            .multilineTextAlignment(TextAlignment.center)
            .frame(height: 60)
            .padding()
            .textFieldStyle(.roundedBorder)
            .border(.secondary)
            .focused($focusField, equals: .argons)

            Button {
              guard let argons = argons else {
                errorMessage = "Please input the amount"
                return
              }
              let milligons = UInt64(argons * 1_000)
              let isRequest = argonRequestType == "Request"
              let localchain = localchain
              Task {
                do {
                  let file = try await localchain.createArgonFile(
                    isRequesting: isRequest,
                    milligons: milligons
                  )
                  await MainActor.run {
                    argonFile = file
                  }

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
              }
            } label: {
              Label("Send", systemImage: "paperplane")
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .foregroundColor(.white)
            .fontWeight(.bold)
          }
        }

        .navigationBarTitle("\(argonRequestType) Money", displayMode: .inline)
        .toolbar {
          Button("", systemImage: "xmark.circle") { isPresented = false }
        }
        .toolbarBackground(.visible, for: .navigationBar)
        .frame(maxWidth: .infinity, alignment: .center)
        .padding()
      }
    }
    .onAppear {
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
        focusField = .argons
      }
    }
    .onDisappear {
      errorMessage = nil
    }
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
