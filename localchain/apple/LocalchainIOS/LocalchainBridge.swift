//
//  LocalchainBridge.swift
//  LocalchainIOS
//
//  Created by Blake Byrnes on 4/1/24.
//

import BigNumber
import Combine
import Foundation

class LocalchainBridge: ObservableObject {
  @Published var address: String?
  @Published var accountOverview: LocalchainOverview?
  @Published var balance: BInt = .init(0)
  @Published var futureBuyingPower: BInt = .init(0)

  private var timer: Timer?
  var localchain: Localchain?

  func load() async throws {
    if localchain != nil {
      return
    }
    let documentsDirectory = try FileManager.default.url(
      for: .documentDirectory,
      in: .userDomainMask,
      appropriateFor: nil,
      create: true
    )
    let fileURL = documentsDirectory.appendingPathComponent("primary.db")

    let tickerConfig = TickerConfig(
      tickDurationMillis: 60_000,
      channelHoldExpirationTicks: 60,
      ntpPoolUrl: nil
    )
    localchain = try await Localchain.newWithoutMainchain(
      path: fileURL.standardizedFileURL.path,
      tickerConfig: tickerConfig,
      keystorePassword: nil
    )
    let address = try await localchain?.useAccount(
      suri: nil,
      password: nil,
      cryptoScheme: CryptoScheme.sr25519
    )
    await MainActor.run {
      self.address = address
    }
    await updateAccount()
    createSyncTask()
  }

  func sync() async {
    guard let localchain = localchain else {
      return
    }

    do {
      _ = try await localchain.sync()
      await updateAccount()
    } catch {
      print("Error synchronizing localchain \(error)")
    }

    do {
      if await localchain.isConnectedToMainchain() == false {
        try await localchain.connectMainchain(mainchainUrl: Settings.mainchainUrl, timeoutMillis: 10_000)
      }
    } catch let UniffiError.Generic(message) {
      print("Error connecting to mainchain \(message)")
    } catch {
      print("Error connecting to mainchain \(error)")
    }
  }

  private func createSyncTask() {
    guard let localchain = localchain else {
      return
    }
    Task {
      while true {
        await sync()
        let timeToNextTick = localchain.durationToNextTick()
        print("time to next tick \(timeToNextTick)")
        try? await Task.sleep(nanoseconds: timeToNextTick * 1_000_000)
      }
    }
  }

  func updateAccount() async {
    guard let accountOverview = try? await localchain?.accountOverview() else {
      return
    }

    await MainActor.run {
      self.accountOverview = accountOverview
      self.balance = .init(accountOverview.balance) ?? BInt(0)
      self.futureBuyingPower = calculateAnnualCompoundInterest(principal: self.balance, rate: 3.0, years: 100)
    }
  }

  func approveArgonRequest(argonFile: ArgonFileTransfer) async throws {
    guard let localchain = localchain else {
      throw AppError.runtimeError("Localchain not initialized")
    }

    _ = try await localchain.transactions().acceptArgonRequest(argonFile: argonFile.json)
    Task {
      await updateAccount()
    }
  }

  func importArgons(argonFile: ArgonFileTransfer) async throws {
    guard let localchain = localchain else {
      throw AppError.runtimeError("Localchain not initialized")
    }

    _ = try await localchain.transactions()
      .importArgons(argonFile: argonFile.json)
    Task {
      await updateAccount()
    }
  }

  func createArgonFile(isRequesting: Bool, milligons: UInt64) async throws -> ArgonFileTransfer {
    guard let localchain = localchain else {
      throw AppError.runtimeError("Localchain not initialized")
    }
    if isRequesting == false {
      let balance = balance
      if milligons > balance {
        let amount = formatArgons(balance)
        throw AppError.insufficientBalance(balance: amount)
      }
    }
    let txs = localchain.transactions()
    let file = try await (
      isRequesting ?
        txs.request(milligons: String(milligons)) : txs.send(
          milligons: String(milligons),
          to: nil
        )
    )
    Task {
      await updateAccount()
    }

    let amount = formatArgons(BInt(milligons))
    return ArgonFileTransfer(name: "\(isRequesting ? "Request" : "Send") \(amount)", json: file)
  }
}

func formatArgons(_ milligons: String, digits: Int = 2) -> String {
  formatArgons(BInt(milligons) ?? BInt(0), digits: digits)
}

func calculateAnnualCompoundInterest(principal: BInt, rate: Double, years: Int) -> BInt {
  let r = rate / 100.0 // Convert percentage to a decimal
  let compoundInterest = Double(principal.description)! * pow(1 + r, Double(years))
  return BInt(compoundInterest)
}

func currencyFormatter(_ symbol: String = "₳", digits: Int = 2) -> NumberFormatter {
  let formatter = NumberFormatter()
  formatter.numberStyle = .currency
  formatter.roundingMode = .floor
  formatter.currencySymbol = symbol
  formatter.maximumFractionDigits = digits
  formatter.locale = Locale.current
  return formatter
}

func formatArgons(_ milligons: BInt, digits: Int = 2) -> String {
  let balanceDecimal = milligons.toDecimal()

  let actualBalance = balanceDecimal / Decimal(1_000.0)

  let formatter = currencyFormatter("₳", digits: digits)
  guard let formattedBalance = actualBalance.formatted(formatter) else {
    return "Formatting Error"
  }

  return formattedBalance
}

func formatCents(_ milligons: BInt) -> String {
  String(milligons % BInt(1_000) / BInt(10)).padding(toLength: 2, withPad: "0", startingAt: 0)
}

extension BInt {
  func toDecimal() -> Decimal {
    let balanceString = description

    return Decimal(string: balanceString)!
  }
}

extension Decimal {
  func formatted(_ formatter: NumberFormatter) -> String? {
    formatter.string(from: NSDecimalNumber(decimal: self))
  }
}
