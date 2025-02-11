//
//  TramTrackerViewModel.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

@MainActor
class TramTrackerViewModel: ObservableObject {
    
    // MARK: - Published properties
    
    @Published var northBoundPredictedArrivals: [PredictedArrival]?
    @Published var southBoundPredictedArrivals: [PredictedArrival]?
    
    var hasLoaded: Bool { northBoundPredictedArrivals != nil && southBoundPredictedArrivals != nil }
    
    @Published var isLoading: Bool = false
    @Published var errorMessage: String?
    
    var northStopIdentifier: String { StopIdentifier.north }
    var southStopIdentifier: String { StopIdentifier.south }
    
    // MARK: - Properties
    
    private let useCase: TramTrackerUseCasing
    
    // MARK: - Constants
    
    private enum StopIdentifier {
        static let north = "4055"
        static let south = "4155"
    }
    
    // MARK: -  Life-cycle
    
    init(useCase: TramTrackerUseCasing = TramTrackerUseCase()) {
        self.useCase = useCase
    }
    
    // MARK: - Public functions
    
    func loadPredictedArrivals() {
        self.isLoading = true
        self.errorMessage = nil
        
        Task {
            do {
                // TODO: - fetch these in parallel and populate the respective published varss
                // Fetch both north and south and only update UI once both have loaded
                async let fetchedNorthBoundPredictedArrivals = try useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.north)
                async let fetchedSouthBoundPredictedArrivals = try useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.south)
                

            } catch {
                self.errorMessage = "⚠️\nCould not load upcoming trams, please try again"
                self.isLoading = false
            }
        }
    }
    
    func clearPredictedArrivals() {
        self.northBoundPredictedArrivals = nil
        self.southBoundPredictedArrivals = nil
    }
    
}
