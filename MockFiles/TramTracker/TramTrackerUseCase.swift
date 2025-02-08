import Foundation

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
        
        return try await tramTrackerController.fetchPredictedArrivals(forStopId: stopId, token: token)
    }
    
}
