//
//  TramTrackerService.swift
//  TramTrackerSwiftUI
//
//  Created on 16/3/2024.
//

import Foundation

protocol TramTrackerServicing {
    func getDeviceToken() async throws -> String
    func getNextPredictedRoutesCollection(forStopId stopId: String, token: String) async throws -> NextPredictedRoutesCollectionResponse
}

class TramTrackerService {
    
    // MARK: - Properties
    
    private let httpClient: HttpClienting
    private let baseUrlString = "https://ws3.tramtracker.com.au/TramTracker/RestService"
    
    // MARK: - Initialisation
    
    init(httpClient: HttpClienting = HttpClient()) {
        self.httpClient = httpClient
    }
    
}

// MARK: - TramTrackerServicing

extension TramTrackerService: TramTrackerServicing {
    
    func getDeviceToken() async throws -> String {
        guard let url = URL(string: "\(baseUrlString)/GetDeviceToken/?aid=TTIOSJSON&devInfo=HomeTime") else {
            fatalError("Invalid URL for getDeviceToken")
        }
        
        let tokenResponse: DeviceTokenResponse = try await httpClient.fetch(from: url)
        return tokenResponse.responseObject[0].deviceToken
    }
    
    func getNextPredictedRoutesCollection(forStopId stopId: String, token: String) async throws -> NextPredictedRoutesCollectionResponse {
        guard let url = URL(string: "\(baseUrlString)/GetNextPredictedRoutesCollection/\(stopId)/78/false/?aid=TTIOSJSON&cid=2&tkn=\(token)") else {
            throw HttpError.badURL
        }
        
        let nextPredictedRoutesCollectionResponse: NextPredictedRoutesCollectionResponse = try await httpClient.fetch(from: url)
        return nextPredictedRoutesCollectionResponse
    }
    
}
