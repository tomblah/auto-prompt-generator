# AI TODO

Bash script that turns your code TODO's into ChatGPT-friendly questions.

## Input

Instead of spending time writing out a question for ChatGPT, simply type out your question as a `TODO` in your code:

```
// TODO: - Can you produce unit tests for this class?
```

AI TODO identifies all relevant parts of your code (referenced classes, structs, enums etc.) and generates a question that includes the context needed for ChatGPT to answer your question.

## Output

When you run `ai_todo.sh` in your project, it will produce a question similar to:


```
Hey ChatGPT!

Can you scan the following code and locate and action the TODO marked by // TODO: - 

--------------------------------------------------
The contents of TramTrackerUseCase.swift is as follows:

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

    // TODO: - Can you produce unit tests for this class?
    
    func fetchUpcomingPredictedArrivals(forStopId stopId: String) async throws -> [PredictedArrival] {
        try await tramTrackerManager.authenticateIfNeeded()
        guard let token = tramTrackerManager.deviceToken else {
            fatalError("Invalid state: no device token after authentication")
        }
        
        return try await tramTrackerController.fetchPredictedArrivals(forStopId: stopId, token: token)
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

```

NB: the above question has been edited for demonstration purposes and for brevity. The current output differs cosmetically from the above.

## ChatGPT

After running the script, the question will be copied to your clipboard, so all you have to do is paste it into ChatGPT.

NB: ChatGPT-4 (rather than ChatGPT-3.x) is far better for programming-related questions.

## So why is all this needed?

Simple answer: **context**.

Instead of spending a bunch of time writing out out a huge question and copying and pasting in all relevant code, just write a one-line TODO and let AI TODO "contextify" the question for you.


## Caveats

- The script is a work in progress!
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


