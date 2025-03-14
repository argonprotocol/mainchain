// prettier-ignore
/* eslint-disable */
/* auto-generated by NAPI-RS */

const { readFileSync } = require('fs')

let nativeBinding = null
const loadErrors = []

const isMusl = () => {
  let musl = false
  if (process.platform === 'linux') {
    musl = isMuslFromFilesystem()
    if (musl === null) {
      musl = isMuslFromReport()
    }
    if (musl === null) {
      musl = isMuslFromChildProcess()
    }
  }
  return musl
}

const isFileMusl = (f) => f.includes('libc.musl-') || f.includes('ld-musl-')

const isMuslFromFilesystem = () => {
  try {
    return readFileSync('/usr/bin/ldd', 'utf-8').includes('musl')
  } catch {
    return null
  }
}

const isMuslFromReport = () => {
  const report = typeof process.report.getReport === 'function' ? process.report.getReport() : null
  if (!report) {
    return null
  }
  if (report.header && report.header.glibcVersionRuntime) {
    return false
  }
  if (Array.isArray(report.sharedObjects)) {
    if (report.sharedObjects.some(isFileMusl)) {
      return true
    }
  }
  return false
}

const isMuslFromChildProcess = () => {
  try {
    return require('child_process').execSync('ldd --version', { encoding: 'utf8' }).includes('musl')
  } catch (e) {
    // If we reach this case, we don't know if the system is musl or not, so is better to just fallback to false
    return false
  }
}

function requireNative() {
  if (process.platform === 'android') {
    if (process.arch === 'arm64') {
      try {
        return require('./localchain.android-arm64.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-android-arm64')
      } catch (e) {
        loadErrors.push(e)
      }

    } else if (process.arch === 'arm') {
      try {
        return require('./localchain.android-arm-eabi.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-android-arm-eabi')
      } catch (e) {
        loadErrors.push(e)
      }

    } else {
      loadErrors.push(new Error(`Unsupported architecture on Android ${process.arch}`))
    }
  } else if (process.platform === 'win32') {
    if (process.arch === 'x64') {
      try {
        return require('./localchain.win32-x64-msvc.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-win32-x64-msvc')
      } catch (e) {
        loadErrors.push(e)
      }

    } else if (process.arch === 'ia32') {
      try {
        return require('./localchain.win32-ia32-msvc.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-win32-ia32-msvc')
      } catch (e) {
        loadErrors.push(e)
      }

    } else if (process.arch === 'arm64') {
      try {
        return require('./localchain.win32-arm64-msvc.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-win32-arm64-msvc')
      } catch (e) {
        loadErrors.push(e)
      }

    } else {
      loadErrors.push(new Error(`Unsupported architecture on Windows: ${process.arch}`))
    }
  } else if (process.platform === 'darwin') {
    try {
        return require('./localchain.darwin-universal.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-darwin-universal')
      } catch (e) {
        loadErrors.push(e)
      }

    if (process.arch === 'x64') {
      try {
        return require('./localchain.darwin-x64.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-darwin-x64')
      } catch (e) {
        loadErrors.push(e)
      }

    } else if (process.arch === 'arm64') {
      try {
        return require('./localchain.darwin-arm64.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-darwin-arm64')
      } catch (e) {
        loadErrors.push(e)
      }

    } else {
      loadErrors.push(new Error(`Unsupported architecture on macOS: ${process.arch}`))
    }
  } else if (process.platform === 'freebsd') {
    if (process.arch === 'x64') {
      try {
        return require('./localchain.freebsd-x64.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-freebsd-x64')
      } catch (e) {
        loadErrors.push(e)
      }

    } else if (process.arch === 'arm64') {
      try {
        return require('./localchain.freebsd-arm64.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-freebsd-arm64')
      } catch (e) {
        loadErrors.push(e)
      }

    } else {
      loadErrors.push(new Error(`Unsupported architecture on FreeBSD: ${process.arch}`))
    }
  } else if (process.platform === 'linux') {
    if (process.arch === 'x64') {
      if (isMusl()) {
        try {
        return require('./localchain.linux-x64-musl.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-x64-musl')
      } catch (e) {
        loadErrors.push(e)
      }

      } else {
        try {
        return require('./localchain.linux-x64-gnu.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-x64-gnu')
      } catch (e) {
        loadErrors.push(e)
      }

      }
    } else if (process.arch === 'arm64') {
      if (isMusl()) {
        try {
        return require('./localchain.linux-arm64-musl.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-arm64-musl')
      } catch (e) {
        loadErrors.push(e)
      }

      } else {
        try {
        return require('./localchain.linux-arm64-gnu.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-arm64-gnu')
      } catch (e) {
        loadErrors.push(e)
      }

      }
    } else if (process.arch === 'arm') {
      if (isMusl()) {
        try {
        return require('./localchain.linux-arm-musleabihf.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-arm-musleabihf')
      } catch (e) {
        loadErrors.push(e)
      }

      } else {
        try {
        return require('./localchain.linux-arm-gnueabihf.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-arm-gnueabihf')
      } catch (e) {
        loadErrors.push(e)
      }

      }
    } else if (process.arch === 'riscv64') {
      if (isMusl()) {
        try {
        return require('./localchain.linux-riscv64-musl.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-riscv64-musl')
      } catch (e) {
        loadErrors.push(e)
      }

      } else {
        try {
        return require('./localchain.linux-riscv64-gnu.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-riscv64-gnu')
      } catch (e) {
        loadErrors.push(e)
      }

      }
    } else if (process.arch === 'ppc64') {
      try {
        return require('./localchain.linux-ppc64-gnu.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-ppc64-gnu')
      } catch (e) {
        loadErrors.push(e)
      }

    } else if (process.arch === 's390x') {
      try {
        return require('./localchain.linux-s390x-gnu.node')
      } catch (e) {
        loadErrors.push(e)
      }
      try {
        return require('@argonprotocol/localchain-linux-s390x-gnu')
      } catch (e) {
        loadErrors.push(e)
      }

    } else {
      loadErrors.push(new Error(`Unsupported architecture on Linux: ${process.arch}`))
    }
  } else {
    loadErrors.push(new Error(`Unsupported OS: ${process.platform}, architecture: ${process.arch}`))
  }
}

nativeBinding = requireNative()

if (!nativeBinding || process.env.NAPI_RS_FORCE_WASI) {
  try {
    nativeBinding = require('./localchain.wasi.cjs')
  } catch (err) {
    if (process.env.NAPI_RS_FORCE_WASI) {
      console.error(err)
    }
  }
  if (!nativeBinding) {
    try {
      nativeBinding = require('@argonprotocol/localchain-wasm32-wasi')
    } catch (err) {
      if (process.env.NAPI_RS_FORCE_WASI) {
        console.error(err)
      }
    }
  }
}

if (!nativeBinding) {
  if (loadErrors.length > 0) {
    // TODO Link to documentation with potential fixes
    //  - The package owner could build/publish bindings for this arch
    //  - The user may need to bundle the correct files
    //  - The user may need to re-install node_modules to get new packages
    throw new Error('Failed to load native binding', { cause: loadErrors })
  }
  throw new Error(`Failed to load native binding`)
}

module.exports.AccountStore = nativeBinding.AccountStore
module.exports.BalanceChange = nativeBinding.BalanceChange
module.exports.BalanceChangeRow = nativeBinding.BalanceChangeRow
module.exports.BalanceChangeBuilder = nativeBinding.BalanceChangeBuilder
module.exports.BalanceChangeStore = nativeBinding.BalanceChangeStore
module.exports.BalanceSync = nativeBinding.BalanceSync
module.exports.BalanceSyncResult = nativeBinding.BalanceSyncResult
module.exports.BalanceTipResult = nativeBinding.BalanceTipResult
module.exports.ChannelHold = nativeBinding.ChannelHold
module.exports.ChannelHoldResult = nativeBinding.ChannelHoldResult
module.exports.DomainLease = nativeBinding.DomainLease
module.exports.DomainRow = nativeBinding.DomainRow
module.exports.DomainStore = nativeBinding.DomainStore
module.exports.Keystore = nativeBinding.Keystore
module.exports.LocalAccount = nativeBinding.LocalAccount
module.exports.Localchain = nativeBinding.Localchain
module.exports.MainchainClient = nativeBinding.MainchainClient
module.exports.MainchainTransferStore = nativeBinding.MainchainTransferStore
module.exports.NotarizationBuilder = nativeBinding.NotarizationBuilder
module.exports.NotarizationTracker = nativeBinding.NotarizationTracker
module.exports.NotaryClient = nativeBinding.NotaryClient
module.exports.NotaryClients = nativeBinding.NotaryClients
module.exports.OpenChannelHold = nativeBinding.OpenChannelHold
module.exports.OpenChannelHoldsStore = nativeBinding.OpenChannelHoldsStore
module.exports.OverviewStore = nativeBinding.OverviewStore
module.exports.Subscription = nativeBinding.Subscription
module.exports.TickerRef = nativeBinding.TickerRef
module.exports.Transactions = nativeBinding.Transactions
module.exports.AccountType = nativeBinding.AccountType
module.exports.ADDRESS_PREFIX = nativeBinding.ADDRESS_PREFIX
module.exports.ARGON_FILE_VERSION = nativeBinding.ARGON_FILE_VERSION
module.exports.ArgonFileType = nativeBinding.ArgonFileType
module.exports.BalanceChangeStatus = nativeBinding.BalanceChangeStatus
module.exports.Chain = nativeBinding.Chain
module.exports.CHANNEL_HOLD_CLAWBACK_TICKS = nativeBinding.CHANNEL_HOLD_CLAWBACK_TICKS
module.exports.CHANNEL_HOLD_MINIMUM_SETTLEMENT = nativeBinding.CHANNEL_HOLD_MINIMUM_SETTLEMENT
module.exports.CryptoScheme = nativeBinding.CryptoScheme
module.exports.DATASTORE_MAX_VERSIONS = nativeBinding.DATASTORE_MAX_VERSIONS
module.exports.DOMAIN_LEASE_COST = nativeBinding.DOMAIN_LEASE_COST
module.exports.DOMAIN_MIN_NAME_LENGTH = nativeBinding.DOMAIN_MIN_NAME_LENGTH
module.exports.DomainTopLevel = nativeBinding.DomainTopLevel
module.exports.NOTARIZATION_MAX_BALANCE_CHANGES = nativeBinding.NOTARIZATION_MAX_BALANCE_CHANGES
module.exports.NOTARIZATION_MAX_BLOCK_VOTES = nativeBinding.NOTARIZATION_MAX_BLOCK_VOTES
module.exports.NOTARIZATION_MAX_DOMAINS = nativeBinding.NOTARIZATION_MAX_DOMAINS
module.exports.runCli = nativeBinding.runCli
module.exports.TransactionType = nativeBinding.TransactionType
module.exports.TRANSFER_TAX_CAP = nativeBinding.TRANSFER_TAX_CAP
