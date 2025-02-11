//
//  TramTrackerManager.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

protocol TramTrackerManaging {
    var deviceToken: String? { get }
    func authenticateIfNeeded() async throws
}

class TramTrackerManager {
    
    // MARK: - Properties
    
    static let sharedInstance = TramTrackerManager()
    
    private let tramTrackerController: TramTrackerControlling
    private(set) var deviceToken: String?
    
    // MARK: - Initialisation
    
    init(tramTrackerController: TramTrackerControlling = TramTrackerController()) {
        self.tramTrackerController = tramTrackerController
    }
    
}

// MARK: - TramTrackerManaging

extension TramTrackerManager: TramTrackerManaging {
    
    func authenticateIfNeeded() async throws {
        guard deviceToken == nil else { return }
        deviceToken = try await tramTrackerController.fetchDeviceToken()
    }
    
}
