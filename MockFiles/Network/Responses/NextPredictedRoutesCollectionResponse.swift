//
//  NextPredictedRoutesCollectionResponse.swift
//  TramTrackerSwiftUI
//
//  Created on 16/3/2024.
//

import Foundation

struct NextPredictedRoutesCollectionResponse: Codable {
    let errorMessage: String?
    let hasError: Bool
    let hasResponse: Bool
    let timeRequested: String
    let timeResponded: String
    let webMethodCalled: String
    
    let responseObject: [NextPredictedRouteInfo]
}

struct NextPredictedRouteInfo: Codable {
    let routeNo: String
    let predictedArrivalDateTime: String
    let vehicleNo: Int
    let airConditioned: Bool
    
    enum CodingKeys: String, CodingKey {
        case routeNo = "RouteNo"
        case predictedArrivalDateTime = "PredictedArrivalDateTime"
        case vehicleNo = "VehicleNo"
        case airConditioned = "AirConditioned"
    }
}
