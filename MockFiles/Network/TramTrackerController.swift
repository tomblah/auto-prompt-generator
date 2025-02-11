//
//  TramTrackerController.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

enum TramTrackerControllerError: Error {
    case errorDecodingData
}

protocol TramTrackerControlling {
    func fetchDeviceToken() async throws -> String
    func fetchPredictedArrivals(forStopId stopId: String, token: String) async throws -> [PredictedArrival]
}

class TramTrackerController {
    
    // MARK: - Properties
    
    private let tramTrackerService: TramTrackerServicing
    
    // MARK: - Initialisation
    
    init(tramTrackerService: TramTrackerServicing = TramTrackerService()) {
        self.tramTrackerService = tramTrackerService
    }
    
}

// MARK: - TramTrackerControlling

extension TramTrackerController: TramTrackerControlling {
    
    func fetchDeviceToken() async throws -> String {
        return try await tramTrackerService.getDeviceToken()
    }
    
    func fetchPredictedArrivals(forStopId stopId: String, token: String) async throws -> [PredictedArrival] {
        let nextPredictedRoutesCollectionResponse = try await tramTrackerService.getNextPredictedRoutesCollection(forStopId: stopId, token: token)
        
        // TODO: check errors by directly having a look at the JSON, e.g. there's a field "hasError"
        
        // Map responses into business objects
        let predictedArrivals = try nextPredictedRoutesCollectionResponse.responseObject.map { nextPredictedRouteInfo in
            let tram = Tram(vehicleNumber: nextPredictedRouteInfo.vehicleNo, isAirConditioned: nextPredictedRouteInfo.airConditioned)
            guard let predictedArrivalDateTime = self.dateFromDotNetFormattedDateString(nextPredictedRouteInfo.predictedArrivalDateTime) else {
                throw TramTrackerControllerError.errorDecodingData
            }
            let predictedArrival = PredictedArrival(tram: tram, routeNumber: nextPredictedRouteInfo.routeNo, predictedArrivalDateTime: predictedArrivalDateTime)
            return predictedArrival
        }
        
        return predictedArrivals
    }
    
}

// MARK: - Private functions

private extension TramTrackerController {
    
    func dateFromDotNetFormattedDateString(_ string: String) -> Date? {
        guard let startRange = string.range(of: "("), let endRange = string.range(of: "+") else { return nil }
        let lowBound = string.index(startRange.lowerBound, offsetBy: 1)
        let range = lowBound..<endRange.lowerBound
        let dateAsString = string[range]
        guard let time = Double(dateAsString) else { return nil }
        let unixTimeInterval = time / 1000
        return Date(timeIntervalSince1970: unixTimeInterval)
    }
    
}
