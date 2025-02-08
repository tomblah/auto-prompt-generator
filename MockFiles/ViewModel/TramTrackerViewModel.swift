import Foundation

// TODO: - can you have a look at how to improve the threading here?
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
        Task {
            self.isLoading = true
            self.errorMessage = nil
            
            do {
                // Fetch both north and south and only update UI once both have loaded
                let fetchedNorthBoundPredictedArrivals = try await useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.north)
                let fetchedSouthBoundPredictedArrivals = try await useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.south)
                
                await MainActor.run {
                    self.northBoundPredictedArrivals = fetchedNorthBoundPredictedArrivals
                    self.southBoundPredictedArrivals = fetchedSouthBoundPredictedArrivals
                    self.isLoading = false
                }
            } catch {
                await MainActor.run {
                    // TODO: have an enum for error messages
                    self.errorMessage = "⚠️\nCould not load upcoming trams, please try again"
                    self.isLoading = false
                }
            }
        }
    }
    
    func clearPredictedArrivals() {
        self.northBoundPredictedArrivals = nil
        self.southBoundPredictedArrivals = nil
    }
    
}
