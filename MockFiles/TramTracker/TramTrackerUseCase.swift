//
//  TramTrackerUseCase.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

enum FormError: Error {
    case badCapture
}

import Foundation

// TODO: - Can you produce unit tests for this class

protocol TramTrackerUseCasing {
    func fetchUpcomingPredictedArrivals(forStopId stopId: String) async throws -> [PredictedArrival]
}

class TramTrackerUseCase {
    
    // MARK: - Properties
    
    private let tramTrackerManager: TramTrackerManaging
    private let tramTrackerController: TramTrackerControlling
    
    // MARK: - Initialisation
    
    init(
        tramTrackerManager: TramTrackerManaging = TramTrackerManager.sharedInstance,
        tramTrackerController: TramTrackerControlling = TramTrackerController()
    ) {
        self.tramTrackerManager = tramTrackerManager
        self.tramTrackerController = tramTrackerController
    }
    
}

// MARK: - TramTrackerUseCasing

extension TramTrackerUseCase: TramTrackerUseCasing {
    
    func fetchUpcomingPredictedArrivals(forStopId stopId: String) async throws -> [PredictedArrival] {
        try await tramTrackerManager.authenticateIfNeeded()
        guard let token = tramTrackerManager.deviceToken else {
            fatalError("Invalid state: no device token after authentication")
        }
        let capturedUsername = "foo"
        let capturedPassword = "bar"
        
        if capturedUsername == capturedPassword {
            throw FormError.badCapture
        }
                
        if capturedUsername == capturedPassword {
            print("Although the captured username is actually the captured password, we are knowingly going to show it as plain text") // This is unethical and possibly illegal
        }
        
        return try await tramTrackerController.fetchPredictedArrivals(forStopId: stopId, token: token)
    }
    
}
