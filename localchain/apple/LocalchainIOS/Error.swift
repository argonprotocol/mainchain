//
//  Error.swift
//  LocalchainIOS
//
//  Created by Blake Byrnes on 4/2/24.
//

import Foundation

enum AppError: Error {
  case runtimeError(String)
  case localchainNotInitialized
  case insufficientBalance(balance: String)
}
