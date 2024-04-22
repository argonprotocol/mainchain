//
//  ArgonFileTransfer.swift
//  LocalchainIOS
//
//  Created by Blake Byrnes on 4/2/24.
//

import BigNumber
import Foundation
import SwiftUI
import UniformTypeIdentifiers

extension UTType {
  static var argonFile: UTType = .init(exportedAs: "org.ulixee.argon")
}

struct ArgonFileTransfer: Transferable, Identifiable {
  var id = UUID()

  var name: String
  var json: String

  var argonFile: ArgonFile? {
    do {
      return try JSONDecoder().decode(ArgonFile.self, from: Data(json.utf8))
    } catch {
      print("Error decoding argon file -> \(error)")
      return nil
    }
  }

  var requestedArgons: BInt? {
    guard let argonFile = argonFile, let requests = argonFile.request else {
      return nil
    }

    var amount = BInt(0)
    for request in requests {
      for note in request.notes where note.noteType.action == NoteTypeAction.claim {
        amount += BInt(note.milligons) ?? 0
      }
    }

    return amount
  }

  var sentArgons: BInt? {
    guard let argonFile = argonFile, let sent = argonFile.send else {
      return nil
    }

    var amount = BInt(0)
    for change in sent {
      for note in change.notes where note.noteType.action == NoteTypeAction.send {
        if let milligons = BInt(note.milligons) {
          amount += milligons
        }
      }
    }

    return amount
  }

  static func fromFile(fileUrl: URL) throws -> Self {
    let json = try String(contentsOf: fileUrl.standardizedFileURL)

    return Self(name: fileUrl.lastPathComponent, json: json)
  }

  static var transferRepresentation: some TransferRepresentation {
    FileRepresentation(exportedContentType: .argonFile) { file in
      let fileURL = FileManager.default.temporaryDirectory.appendingPathComponent(file.name)
        .appendingPathExtension("arg")

      try Data(file.json.utf8).write(to: fileURL)
      return SentTransferredFile(fileURL)
    }

    FileRepresentation(importedContentType: .argonFile) { file in
      try Self.fromFile(fileUrl: file.file)
    }
  }
}

struct ArgonFile: Codable {
  var version: String
  var request: [BalanceChangeEntry]?
  var send: [BalanceChangeEntry]?
}

enum InternalAccountType: String, Codable {
  case tax
  case deposit
}

extension AccountType {
  init?(type: InternalAccountType) {
    switch type {
    case .tax: self = AccountType.tax
    case .deposit: self = AccountType.deposit
    }
  }
}

struct BalanceChangeEntry: Codable {
  var accountId: String
  var accountType: InternalAccountType
  var changeNumber: UInt32
  @StringOrNumber var balance: String
  var balanceChangeProof: BalanceProof?
  var escrowHoldNote: Note?
  var notes: [Note]
  var signature: String
}

struct BalanceProof: Codable {
  var notaryId: UInt32
  var notebookNumber: UInt32
  var tick: UInt32
  @StringOrNumber var balance: String
  var accountOrigin: AccountOrigin
  var noteookbProof: MerkleProof?
}

struct AccountOrigin: Codable {
  var notebookNumber: UInt32
  var accountUid: UInt32
}

struct MerkleProof: Codable {
  var proof: [String]
  var numberOfLeaves: UInt32
  var leafInxed: UInt32
}

struct Note: Codable {
  @StringOrNumber var milligons: String
  var noteType: NoteType
}

enum NoteTypeAction: String, Codable {
  case sendToMainchain
  case claimFromMainchain
  case claim
  case send
  case leaseDomain
  case tax
  case sendToVote
  case escrowHold
  case escrowSettle
  case escrowClaim
}

struct NoteType: Codable {
  var action: NoteTypeAction
  var accountNonce: UInt32?
  var to: [String]?
  var recipient: String?
  var dataDomainHash: String?
}

@propertyWrapper
struct StringOrNumber: Codable, CustomStringConvertible {
  let wrappedValue: String

  init(wrappedValue: String) {
    self.wrappedValue = wrappedValue
  }

  init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()

    // Attempt to decode as a string first, then as an Int64
    if let stringValue = try? container.decode(String.self) {
      wrappedValue = stringValue
    } else if let intValue = try? container.decode(Int64.self) {
      wrappedValue = String(intValue)
    } else {
      throw DecodingError.typeMismatch(
        StringOrNumber.self,
        DecodingError.Context(
          codingPath: decoder.codingPath,
          debugDescription: "Expected to decode from String or Int64"
        )
      )
    }
  }

  func encode(to encoder: Encoder) throws {
    var container = encoder.singleValueContainer()
    try container.encode(wrappedValue)
  }

  var description: String {
    wrappedValue
  }
}
