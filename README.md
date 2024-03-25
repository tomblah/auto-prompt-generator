# AI TODO

Turns your `TODO`'s into AI-friendly questions.

(Beta: Currently only available for ChatGPT and only for Swift questions. It is very much a work in progress.)

## Usage

Download `ai_todo.sh` and place it anywhere in a Swift project that has a github repo.

Write a question for ChatGPT, something like this:

```
// TODO: - Can you produce unit tests for this class?
```

In one of your files, e.g.:

```
protocol TramTrackerUseCasing {
    func fetchUpcomingPredictedArrivals(forStopId stopId: String) async throws -> [PredictedArrival]
}

// TODO: - Can you produce unit tests for this class?

class TramTrackerUseCase {
    
    private let tramTrackerManager: TramTrackerManaging
    private let tramTrackerController: TramTrackerControlling
    
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
    
    // ... rest of class ...
    
}
```

And then run the script! 

The script will go through the Swift file containing the `TODO: - ` and identify classes, protocols, enums etc. Then it will attempt to find where those are defined in your project, and include the contents of those files with your ChatGPT question. This should provide ChatGPT with the **sufficient context it needs** in order to correctly answer your question.

A typical generated question will look something like:


```
The contents of PredictedArrival.swift is as follows:

import Foundation

struct PredictedArrival {
    let tram: Tram
    let routeNumber: String
    let predictedArrivalDateTime: Date
}

--------------------------------------------------
The contents of TramTrackerController.swift is as follows:

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

--------------------------------------------------
The contents of TramTrackerManager.swift is as follows:

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

--------------------------------------------------
The contents of TramTrackerUseCase.swift is as follows:

import Foundation

// TODO: ChatGPT: Can you produce unit tests for this class

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

--------------------------------------------------

Can you do the TODO: ChatGPT: in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by // TODO: ChatGPT:

```


It then copies the question to the clipboard, so you can simply paste into ChatGPT.

## So why is all this needed?

Simple answer: **context**.

ChatGPT-4 will produce correct and useful code 95% of the time if you:

- (a) state your question clearly, and,

- (b) provide sufficient context such that ChatGPT understands your question.

However...

...identifying code snippets that are relevant to your question and copying and pasting them in is **time-consuming** and is just a drag. And it's not like you can simply paste in your entire project and click "go"!

AI TODO takes the pain out of creating the context for ChatGPT questions.

## Caveats

- Swift and ChatGPT only at the moment,
- Don't use ChatGPT-3.x, it doesn't work well for programming questions the last time I checked. Use ChatGPT-4,
- You must write your question like `// TODO: - ` i.e. with the hyphen (or `// TODO: ChatGPT: `). This is so it doesn't go crazy and locate all your `TODO`s in your project,
- The script will automatically convert `// TODO: - ` into `// TODO: ChatGPT: ` when copying to your clipboard. This is because I figure that `// TODO: ChatGPT: ` has less potential for confusion for ChatGPT (although I'm probably being overly cautious),
- One question at a time. Getting ChatGPT to do multiple questions at a time can confuse it. Keep it simple. Will throw an error if there's multiple questions,
- The script is a bit clunky. The logic for identifying "types" (i.e. classes, protocols, enums etc.) is rudimentary. It just looks for capitilized words, then searches for where they are defined in code,
- Produces a bunch pollution i.e. temporary files for debugging,
- I am not a bash programmer. I got ChatGPT to make this script. Although it works for me, there's probably a ton of bugs in it.

## Future work

This script has `1.5x`'d my output. I'd love to see other people adapt it for their purposes e.g. adding support for other languages.


