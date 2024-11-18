//
//  ContentView.swift
//  LocalchainIOS
//
//  Created by Blake Byrnes on 3/29/24.
//

import BigNumber
import SwiftUI
import UniformTypeIdentifiers

struct PrimaryButton: ButtonStyle {
  func makeBody(configuration: Configuration) -> some View {
    configuration.label
      .padding()
      .background(.accent)
      .frame(maxWidth: .infinity)
      .fontWeight(.bold)
      .foregroundStyle(.white)
      .scaleEffect(configuration.isPressed ? 1.2 : 1)
      .animation(.easeOut(duration: 0.2), value: configuration.isPressed)
  }
}

struct CurvedBackground: Shape {
  func path(in rect: CGRect) -> Path {
    var path = Path()

    // Start at the top left
    path.move(to: CGPoint(x: rect.minX, y: rect.minY))
    // Draw a line to the top right
    path.addLine(to: CGPoint(x: rect.maxX, y: rect.minY))
    // Draw a line to the bottom right
    path.addLine(to: CGPoint(x: rect.maxX, y: rect.maxY))

    // Draw the bottom curve
    path.addCurve(
      to: CGPoint(x: rect.minX, y: rect.maxY), // End at the bottom left
      control1: CGPoint(x: rect.maxX - rect.width * 0.25, y: rect.maxY + rect.height * 0.4), // Control point 1
      control2: CGPoint(x: rect.minX + rect.width * 0.25, y: rect.maxY + rect.height * 0.4) // Control point 2
    )

    // Close the path
    path.closeSubpath()
    return path
  }
}

let lightpurple = Color("lightpurple")
let magenta = Color(UIColor(red: 147.0 / 255.0, green: 33 / 255.0, blue: 166 / 255.0, alpha: 1))
let bgray = Color(#colorLiteral(red: 0.9365568161, green: 0.9451275468, blue: 0.9492741227, alpha: 1))
let darkbutton = Color(#colorLiteral(red: 0.6335534453, green: 0.3463019729, blue: 0.7147291303, alpha: 1))
let fadedbutton = Color(UIColor(red: 153 / 255.0, green: 68 / 255.0, blue: 176 / 255.0, alpha: 1))

struct ContentView: View {
  @StateObject var localchainLoader = LocalchainBridge()
  @State var addressText = "Loading your account"
  @State var errorText: String?
  @State var showError = false
  @State var showArgonRequestModalView: Bool = false
  @State var showSettingsModalView: Bool = false
  @State var showArgonFileModalView: Bool = false
  @State var argonFileTransfer: ArgonFileTransfer?
  @State var showQrScanner: Bool = false
  @State var showBuyingPower: Bool = false
  @State var toggle = "dashboard"

  let dollarsFormatter = currencyFormatter("$", digits: 0)

  var body: some View {
    NavigationStack {
      VStack(alignment: .center, spacing: 10) {
        HStack {
          Spacer()

          Button(action: {
            showSettingsModalView = true
          }) {
            Image("settings")
              .resizable()
              .frame(width: 20, height: 20)
              .foregroundColor(.white)
          }
          .padding(.trailing, 20)
          .padding(.top, 5)
        }

        Spacer()
        Text("Your Balance")
          .font(.headline)
          .fontWeight(.thin)
          .foregroundColor(.white.opacity(0.9))
          .padding(.bottom, 0)

        HStack(spacing: 2) {
          Text("\(formatArgons(localchainLoader.balance, digits: 0))")
            .fontWeight(.heavy)
            .font(.system(size: 80.0))
            .foregroundColor(.white)
          Text(
            "\(formatCents(localchainLoader.balance))"
          )
          .bold()
          .font(.system(size: 42.0))
          .foregroundColor(.white)
          .baselineOffset(28.0)
        }

        CustomDivider()
          .padding(.vertical, 20)

        Text("Argon is an inflation-proof stablecoin that uses sound money principles to ensure long-term stability.")
          .font(.subheadline)
          .fontWeight(.light)
          .foregroundColor(.white.opacity(0.7))
          .multilineTextAlignment(.center)
          .padding(.horizontal, 15)
          .padding(.bottom, 15)
          .padding(.top, 15)

        if self.showBuyingPower == true {
          HStack {
            Spacer()
            Spacer()

            Button {
              showBuyingPower = false
            } label: {
              Label("", systemImage: "xmark")
            }
            .foregroundColor(.white)
            .padding(.trailing, 5)
          }.padding(.bottom, -15)

          Text("Future Buying Power*")
            .foregroundColor(Color.white)
            .fontWeight(.light)
            .padding(.bottom, -10)

          HStack(spacing: 2) {
            Text(
              "\((localchainLoader.futureBuyingPower.toDecimal() / Decimal(1_000_000.0)).formatted(dollarsFormatter) ?? "Err")"
            )
            .fontWeight(.bold)
            .font(.system(size: 40.0))
            .foregroundColor(.white)
            Text(
              "\(formatCents(localchainLoader.futureBuyingPower))"
            )
            .bold()
            .font(.system(size: 18.0))
            .foregroundColor(.white)
            .baselineOffset(16.0)
          }

          HStack {
            Button {}
              label: { Text("Calculator")
                .frame(maxWidth: .infinity)
                .foregroundColor(.white)
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
              }
              .background(
                RoundedRectangle(cornerRadius: 3, style: .continuous)
                  .stroke(.lightpurple, lineWidth: 1)
              )

            Button {}
              label: { Text("Learn More")
                .frame(maxWidth: .infinity)
                .foregroundColor(.white)
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
              }
              .background(
                RoundedRectangle(cornerRadius: 3, style: .continuous)
                  .stroke(.lightpurple, lineWidth: 1)
              )
          }
          .padding(.horizontal, 25)
        } else {
          Button {
            self.showBuyingPower = true
          }
          label: { Text("View My Buying Power")
            .font(.subheadline)
            .fontWeight(.light)
            .foregroundColor(.white)
            .background(
              VStack {
                Spacer()
                DashedUnderline()
                  .frame(height: 1)
                  .offset(y: 5)
              }
            )
          }
          .padding(.bottom, 10)
          .padding(.horizontal, 20)
        }

        CustomDivider()
          .padding(.vertical, 20)

        Spacer()

        HStack(spacing: 2) {
          Button {
            showArgonRequestModalView = true
          } label: {
            Label("Send", image: "send")
              .frame(maxWidth: .infinity)
              .fontWeight(.bold)
              .font(.headline)
              .padding(.vertical, 10)
          }
          .background(.lightpurple)
          .cornerRadius(8)
          .overlay(
            RoundedRectangle(cornerRadius: 8)
              .stroke(Color.black.opacity(0.8), lineWidth: 0.6)
          )
          .foregroundColor(.white)
          .fontWeight(.bold)

          Spacer()

          Button {
            showQrScanner = true
          } label: {
            Label("Receive", image: "receive")
              .frame(maxWidth: .infinity)
              .fontWeight(.bold)
              .padding(.vertical, 10)
          }
          .background(darkbutton)
          .cornerRadius(8)
          .overlay(
            RoundedRectangle(cornerRadius: 8)
              .stroke(Color.black.opacity(0.8), lineWidth: 0.6)
          )
          .foregroundColor(.white)
        }.padding(.horizontal, 10)
          .padding(.bottom, 20)
        //
        //        Button {}
        //          label: { Text("Open Transaction History")
        //            .frame(maxWidth: .infinity)
        //            .foregroundColor(.white)
        //            .font(.subheadline)
        //            .fontWeight(.light)
        //          }
        //
        //        Spacer()
      }
      .padding(.horizontal, 10)
      .background(gradientBackground)
      .preferredColorScheme(.dark)
    }
    .task {
      do {
        try await localchainLoader.load()

        if let address = localchainLoader.address {
          addressText = "Your Address \(address)"
        }
      } catch let UniffiError.Generic(message) {
        print("Failed to create directory for Localchain \(message)")
        errorText = message
      } catch {
        print("Failed to create directory for Localchain \(error)")
        errorText = "\(error)"
      }
    }
    .sheet(isPresented: $showArgonRequestModalView) {
      ArgonSendSheet(
        isPresented: $showArgonRequestModalView
      )
      .preferredColorScheme(.light)
    }
    .sheet(isPresented: $showArgonFileModalView) {
      ArgonReceivedSheet(
        isPresented: $showArgonFileModalView,
        argonFileTransfer: $argonFileTransfer
      )
      .onDisappear {
        argonFileTransfer = nil
      }
      .preferredColorScheme(.light)
    }
    .sheet(isPresented: $showQrScanner) {
      NavigationView {
        QRScanner(isPresented: $showQrScanner) { message in
          DispatchQueue.main.async {
            showArgonFileModalView = true
            argonFileTransfer = ArgonFileTransfer(name: "", json: message)
          }
        }
        .navigationBarTitle(
          "Load an Argon QR Code",
          displayMode: .inline
        )
        .toolbar {
          Button("", systemImage: "xmark.circle") { showQrScanner = false }
        }
        .toolbarBackground(.visible, for: .navigationBar)
        .preferredColorScheme(.light)
      }
    }
    .onOpenURL { url in
      do {
        let file = try ArgonFileTransfer.fromFile(fileUrl: url)
        argonFileTransfer = file
        showArgonFileModalView = true

      } catch {
        errorText = "Couldn't open the argon file: \(error)"
        showError = true
      }
    }
    .environmentObject(localchainLoader)
    .alert(
      "An error has occurred",
      isPresented: $showError
    ) {}
    message: {
      Text(errorText ?? "Unknown error")
    }
  }

  var gradientBackground: some View {
    LinearGradient(
      gradient: Gradient(colors: [
        Color(uiColor: hexStringToUIColor(hex: "#AD4EC7")),
        Color(uiColor: hexStringToUIColor(hex: "#8C2FA6"))
      ]),
      startPoint: .topLeading,
      endPoint: .bottomTrailing
    )
    .edgesIgnoringSafeArea(.all)
  }
}

func hexStringToUIColor(hex: String) -> UIColor {
  var cString: String = hex.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()

  if cString.hasPrefix("#") {
    cString.remove(at: cString.startIndex)
  }

  if (cString.count) != 6 {
    return UIColor.gray
  }

  var rgbValue: UInt64 = 0
  Scanner(string: cString).scanHexInt64(&rgbValue)

  return UIColor(
    red: CGFloat((rgbValue & 0xFF0000) >> 16) / 255.0,
    green: CGFloat((rgbValue & 0x00FF00) >> 8) / 255.0,
    blue: CGFloat(rgbValue & 0x0000FF) / 255.0,
    alpha: CGFloat(1.0)
  )
}

struct DashedUnderline: View {
  var body: some View {
    GeometryReader { geometry in
      Path { path in
        let dashWidth: CGFloat = 5
        let dashSpacing: CGFloat = 5
        let totalWidth = geometry.size.width
        var currentX: CGFloat = 0

        while currentX < totalWidth {
          path.move(to: CGPoint(x: currentX, y: 0))
          path.addLine(to: CGPoint(x: currentX + dashWidth, y: 0))
          currentX += dashWidth + dashSpacing
        }
      }
      .stroke(Color.white.opacity(0.7), lineWidth: 1)
    }
  }
}

struct CustomDivider: View {
  var body: some View {
    VStack(spacing: 0) {
      Rectangle()
        .fill(Color.black.opacity(0.7))
        .frame(height: 0.8)
      Rectangle()
        .fill(Color.white.opacity(0.5))
        .frame(height: 0.5)
    }
    .frame(height: 2)
  }
}

#Preview {
  ContentView()
}
