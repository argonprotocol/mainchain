//
//  ArgonReceivedSheet.swift
//  LocalchainIOS
//
//  Created by Blake Byrnes on 4/2/24.
//

import BigNumber
import Foundation
import SwiftUI

struct ArgonReceivedSheet: View {
  @Binding var isPresented: Bool
  @Binding var argonFileTransfer: ArgonFileTransfer?
  @EnvironmentObject var localchain: LocalchainBridge
  @State var errorMessage: String?

  var body: some View {
    NavigationView {
      VStack(spacing: 50) {
        if errorMessage != nil {
          Text("\(errorMessage ?? "An error occurred")")
            .foregroundColor(.red)
        }
        if argonFileTransfer?.argonFile?.send?.count ?? 0 > 0 {
          HStack(spacing: 2) {
            Text("\(formatArgons(argonFileTransfer?.sentArgons ?? BInt(0), digits: 0))")
              .fontWeight(.heavy)
              .font(.system(size: 40.0))
              .foregroundColor(.accentColor)
            Text(
              "\(formatCents(argonFileTransfer?.sentArgons ?? BInt(0)))"
            )
            .bold()
            .font(.system(size: 18.0))
            .foregroundColor(.accentColor)
            .baselineOffset(16.0)
          }

          Button {
            let localchain = localchain
            Task {
              do {
                try await localchain.importArgons(argonFile: argonFileTransfer!)
                self.isPresented = false
              } catch let UniffiError.Generic(message) {
                await MainActor.run {
                  errorMessage = message
                }
              } catch {
                print("Error accepting funds \(error)")
                await MainActor.run {
                  errorMessage = "\(error)"
                }
              }
            }
          } label: {
            Label("Accept Funds", systemImage: "square.and.arrow.down")
              .frame(maxWidth: .infinity)
          }
          .buttonStyle(.borderedProminent)
          .foregroundColor(.white)
          .fontWeight(.bold)
        } else if let argonFileTransfer = argonFileTransfer {
          HStack(spacing: 2) {
            Text("\(formatArgons(argonFileTransfer.requestedArgons ?? BInt(0), digits: 0))")
              .fontWeight(.heavy)
              .font(.system(size: 40.0))
              .foregroundColor(.accentColor)
            Text(
              "\(formatCents(argonFileTransfer.requestedArgons ?? BInt(0)))"
            )
            .bold()
            .font(.system(size: 18.0))
            .foregroundColor(.accentColor)
            .baselineOffset(16.0)
          }
          Button {
            Task {
              do {
                try await localchain.approveArgonRequest(argonFile: argonFileTransfer)
                self.isPresented = false
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
            Label("Approve Request", systemImage: "signature")
              .frame(maxWidth: .infinity)
          }
          .buttonStyle(.borderedProminent)
          .foregroundColor(.white)
          .fontWeight(.bold)
        } else {
          Text("No argon file is loaded")
            .foregroundColor(.red)
        }
      }
      .padding()
      .onAppear {
        errorMessage = nil
      }
      .navigationBarTitle(
        "\(argonFileTransfer?.argonFile?.send?.count ?? 0 > 0 ? "Money Received" : "Request to Send Money")",
        displayMode: .inline
      )
      .toolbar {
        Button("", systemImage: "xmark.circle") { isPresented = false }
      }
      .toolbarBackground(.visible, for: .navigationBar)
    }
  }
}
