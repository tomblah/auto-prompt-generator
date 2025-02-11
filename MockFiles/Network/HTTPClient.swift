//
//  HTTPClient.swift
//  TramTrackerSwiftUI
//
//  Created on 16/3/2024.
//

import Foundation

// MARK: - URLSessionProvider

protocol URLSessionProvider {
    func data(for request: URLRequest, delegate: (any URLSessionTaskDelegate)?) async throws -> (Data, URLResponse)
}

extension URLSessionProvider {
    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        return try await data(for: request, delegate: nil)
    }
}

extension URLSession: URLSessionProvider {
    func data(for request: URLRequest, delegate: (any URLSessionTaskDelegate)?) async throws -> (Data, URLResponse) {
        let (data, response) = try await data(for: request)
        return (data, response)
    }
}

enum HttpError: Error {
    case badURL, badResponse, errorDecodingData
}

protocol HttpClienting {
    func fetch<T: Codable>(from url: URL) async throws -> T
}

class HttpClient {
    
    // MARK: - Properties
    
    private let urlSession: URLSessionProvider
    
    // MARK: - Initialization
    
    init(urlSession: URLSessionProvider = URLSession.shared) {
        self.urlSession = urlSession
    }
    
}

// MARK: - HttpClienting

extension HttpClient: HttpClienting {
    
    func fetch<T: Codable>(from url: URL) async throws -> T {
        let (data, response) = try await urlSession.data(from: url)
        
        guard let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 200 else {
            throw HttpError.badResponse
        }
        
        do {
            let decodedObject = try JSONDecoder().decode(T.self, from: data)
            return decodedObject
        } catch {
            throw HttpError.errorDecodingData
        }
    }
    
}

