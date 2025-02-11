//
//  DeviceTokenResponse.swift
//  TramTrackerSwiftUI
//
//  Created on 16/3/2024.
//

import Foundation

struct DeviceTokenResponse: Codable {
    let errorMessage: String?
    let hasError: Bool
    let hasResponse: Bool
    let timeRequested: String
    let timeResponded: String
    let webMethodCalled: String
    
    let responseObject: [DeviceTokenInfo]
}

struct DeviceTokenInfo: Codable {
    let deviceToken: String
    
    enum CodingKeys: String, CodingKey {
        case deviceToken = "DeviceToken"
    }
}
