//
//  Settings.swift
//  LocalchainIOS
//
//  Created by Blake Byrnes on 4/2/24.
//

import Foundation

enum Settings {
  static var mainchainUrl: String {
    #if DEBUG
      #if targetEnvironment(simulator)
        return "ws://localhost:9944"
      #else
        return "wss://husky-witty-highly.ngrok-free.app"
      #endif
    #else
      return "publicnode"
    #endif
  }
}
