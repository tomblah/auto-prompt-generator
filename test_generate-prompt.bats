#!/usr/bin/env bats
# test_generate-prompt.bats
#
# These tests run the main generate-prompt.sh script in a simulated Git repository.
# They verify that (a) when a valid TODO instruction exists, the prompt is assembled
# (and “copied” to our dummy clipboard file), (b) that the script fails when no valid
# TODO instruction is present, (c) that the --slim and --exclude options work as expected,
# (d) that the --singular option causes only the TODO file to be included, (e) that the
# new --force-global option causes the script to ignore package boundaries, and
# (f) that our new Rust-based helpers (for extracting the enclosing type and finding
# referencing files) work as expected.

setup() {
  # Create a temporary directory that will serve as our fake repository.
  TMP_DIR=$(mktemp -d)
 
  # Create a dummy "pbcopy" executable so that our script does not touch the real clipboard.
  mkdir -p "$TMP_DIR/dummybin"
  cat << 'EOF' > "$TMP_DIR/dummybin/pbcopy"
#!/bin/bash
# Write the clipboard content to a file named "clipboard.txt" in the current directory.
cat > clipboard.txt
EOF
  chmod +x "$TMP_DIR/dummybin/pbcopy"
  # Prepend dummybin to PATH so that pbcopy is overridden.
  export PATH="$TMP_DIR/dummybin:$PATH"
 
  # Copy the main script and all its dependency components to TMP_DIR.
  # (This assumes your test files and these scripts are in the same directory;
  # adjust the source paths if necessary.)
  cp "${BATS_TEST_DIRNAME}/generate-prompt.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/find-definition-files.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/assemble-prompt.sh" "$TMP_DIR/"
  cp -r "${BATS_TEST_DIRNAME}/rust" "$TMP_DIR/"
 
  # Change to TMP_DIR (this will become our repository root).
  cd "$TMP_DIR"
 
  # Initialize a Git repository.
  git init -q .
 
  # Create a Swift file with a valid TODO instruction.
  cat << 'EOF' > Test.swift
import Foundation
// TODO: - Test instruction for prompt
class TestClass {}
EOF
 
  # Create an extra Swift file that would normally be discovered for type definitions.
  cat << 'EOF' > Another.swift
struct AnotherStruct {}
EOF
}
 
teardown() {
  rm -rf "$TMP_DIR"
}
 
@test "generate-prompt.sh outputs success message and assembles prompt with fixed instruction" {
  # Run the main script.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]
 
  # Check that the output includes a success message and the fixed instruction.
  [[ "$output" == *"Success:"* ]]
  [[ "$output" == *"Can you do the TODO:- in the above code?"* ]]
 
  # Check that our dummy pbcopy created a clipboard file and that it contains prompt details.
  [ -f "clipboard.txt" ]
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"The contents of Test.swift is as follows:"* ]]
  [[ "$clipboard_content" == *"TestClass"* ]]
}
 
@test "generate-prompt.sh fails when no valid TODO instruction is present" {
  # Remove the valid TODO instruction from Test.swift.
  cat << 'EOF' > Test.swift
import Foundation
class TestClass {}
EOF
 
  run bash generate-prompt.sh
  [ "$status" -ne 0 ]
  [[ "$output" == *"Error:"* ]]
}
 
@test "generate-prompt.sh excludes files specified with --exclude" {
  # Create an extra file to be excluded.
  cat << 'EOF' > ExcludeMe.swift
import Foundation
class ExcludeMe {}
EOF
 
  # Run the script with --exclude option.
  run bash generate-prompt.sh --exclude ExcludeMe.swift
  [ "$status" -eq 0 ]
 
  # Debugging output: print the complete output for inspection.
  echo "DEBUG OUTPUT:"
  echo "$output"
 
  # Extract the final list of files from the output.
  # This extracts the lines between "Files (final list):" and the next separator line.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  echo "DEBUG: Final list of files:" "$final_list" >&2
 
  # Verify that the final list of files does not include ExcludeMe.swift.
  [[ "$final_list" != *"ExcludeMe.swift"* ]]
}
 
@test "generate-prompt.sh singular mode includes only the TODO file" {
  # Create an additional extra file that would normally be processed.
  cat << 'EOF' > Extra.swift
import Foundation
struct ExtraStruct {}
EOF
 
  # Run the script with the --singular flag.
  run bash generate-prompt.sh --singular
  [ "$status" -eq 0 ]
 
  # Check that the output indicates singular mode.
  [[ "$output" == *"Singular mode enabled: only including the TODO file"* ]]
 
  # Extract the final list of file basenames.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
 
  # In singular mode, only the TODO file (Test.swift) should be listed.
  [ "$final_list" = "Test.swift" ]
 
  # Verify that the clipboard content (from dummy pbcopy) includes only Test.swift.
  [ -f "clipboard.txt" ]
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"The contents of Test.swift is as follows:"* ]]
  [[ "$clipboard_content" != *"Another.swift"* ]]
  [[ "$clipboard_content" != *"Extra.swift"* ]]
}
 
@test "generate-prompt.sh singular mode ignores non-TODO files even when present" {
  # Create another extra Swift file that would normally be considered.
  cat << 'EOF' > IgnoreMe.swift
import Foundation
class IgnoreMe {}
EOF
 
  # Run the script with the --singular flag.
  run bash generate-prompt.sh --singular
  [ "$status" -eq 0 ]
 
  # Verify that the final file list printed includes only Test.swift.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
  [ "$final_list" = "Test.swift" ]
 
  # Also check that the assembled prompt in clipboard.txt does not mention IgnoreMe.swift.
  [ -f "clipboard.txt" ]
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Test.swift"* ]]
  [[ "$clipboard_content" != *"IgnoreMe.swift"* ]]
}
 
@test "generate-prompt.sh does not include Swift files from .build directories" {
  # Create a Swift file inside a .build directory that should be ignored.
  mkdir -p ".build/ThirdParty"
  cat << 'EOF' > ".build/ThirdParty/ThirdParty.swift"
import Foundation
class ThirdPartyClass {}
EOF

  # Also create a normal Swift file to be processed.
  cat << 'EOF' > Normal.swift
import Foundation
class NormalClass {}
EOF

  # Ensure Test.swift (with the valid TODO instruction) is reset.
  cat << 'EOF' > Test.swift
import Foundation
// TODO: - Test instruction for prompt
class TestClass {}
EOF

  # Run the main script.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  # Extract the final list of files from the output.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  
  # Assert that the list includes Normal.swift and does not include ThirdParty.swift.
  [[ "$final_list" == *"Normal.swift"* ]]
  [[ "$final_list" != *"ThirdParty.swift"* ]]

  # Also check that the assembled prompt (in clipboard.txt) does not include ThirdParty.swift.
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Normal.swift"* ]]
  [[ "$clipboard_content" != *"ThirdParty.swift"* ]]
}
 
@test "generate-prompt.sh does not include Swift files from Pods directories" {
  # Create a Swift file inside a Pods directory that should be ignored.
  mkdir -p "Pods/SubDir"
  cat << 'EOF' > "Pods/SubDir/PodsFile.swift"
import Foundation
class PodsClass {}
EOF

  # Also create a normal Swift file to be processed.
  cat << 'EOF' > Normal.swift
import Foundation
class NormalClass {}
EOF

  # Ensure Test.swift (with the valid TODO instruction) is reset.
  cat << 'EOF' > Test.swift
import Foundation
// TODO: - Test instruction for prompt
class TestClass {}
EOF

  # Run the main script.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  # Extract the final list of files from the output.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  
  # Assert that the list includes Normal.swift and does not include PodsFile.swift.
  [[ "$final_list" == *"Normal.swift"* ]]
  [[ "$final_list" != *"PodsFile.swift"* ]]

  # Also check that the assembled prompt (in clipboard.txt) does not include PodsFile.swift.
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Normal.swift"* ]]
  [[ "$clipboard_content" != *"PodsFile.swift"* ]]
}
 
@test "generate-prompt.sh uses package root when available without --force-global" {
  # Create a subdirectory "PackageDir" and simulate a package.
  mkdir -p "PackageDir"
  cat << 'EOF' > PackageDir/Package.swift
// Package.swift content
EOF
  # Move Test.swift into the package directory.
  mv Test.swift PackageDir/Test.swift

  # Run the script normally.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]
  # Check that output contains "Found package root:" with "PackageDir"
  [[ "$output" == *"Found package root:"* ]]
  [[ "$output" == *"PackageDir"* ]]
}
 
@test "generate-prompt.sh with --force-global ignores package boundaries" {
  # Create a subdirectory "PackageDir" and simulate a package.
  mkdir -p "PackageDir"
  cat << 'EOF' > PackageDir/Package.swift
// Package.swift content
EOF
  # Move Test.swift into the package directory.
  mv Test.swift PackageDir/Test.swift

  # Run the script with --force-global.
  run bash generate-prompt.sh --force-global
  [ "$status" -eq 0 ]
  # Check that output contains the force global enabled message.
  [[ "$output" == *"Force global enabled: ignoring package boundaries and using Git root for context."* ]]
  # And it should not display the package root message.
  [[ "$output" != *"Found package root:"* ]]
}
 
@test "generate-prompt.sh outputs correct file list and success message with realistic file content" {
# Remove any default Swift files created by the standard setup.
rm -f Test.swift Another.swift

# Create the directory structure matching your repository layout.
mkdir -p MockFiles/Model
mkdir -p MockFiles/TramTracker
mkdir -p MockFiles/ViewModel

# Create a realistic TramTrackerViewModel.swift file in MockFiles/ViewModel/
# (This file contains the unique TODO instruction that the script should select.)
cat << 'EOF' > MockFiles/ViewModel/TramTrackerViewModel.swift
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
 
 // MARK: - Properties
 
 private let useCase: TramTrackerUseCasing
 
 // MARK: - Constants
 
 private enum StopIdentifier {
     static let north = "4055"
     static let south = "4155"
 }
 
 // MARK: - Life-cycle
 
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
             async let fetchedNorthBoundPredictedArrivals = try useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.north)
             async let fetchedSouthBoundPredictedArrivals = try useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.south)
             // (Rest of implementation omitted for brevity)
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
EOF

# Create a realistic TramTrackerUseCase.swift file in MockFiles/TramTracker/
# (This file contains an alternate TODO that should be ignored.)
cat << 'EOF' > MockFiles/TramTracker/TramTrackerUseCase.swift
//
//  TramTrackerUseCase.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

protocol TramTrackerUseCasing {
 func fetchUpcomingPredictedArrivals(forStopId stopId: String) async throws -> [PredictedArrival]
}

class TramTrackerUseCase: TramTrackerUseCasing {
 
 private let tramTrackerManager: TramTrackerManaging
 private let tramTrackerController: TramTrackerControlling
 
 init(
     tramTrackerManager: TramTrackerManaging = TramTrackerManager.sharedInstance,
     tramTrackerController: TramTrackerControlling = TramTrackerController()
 ) {
     self.tramTrackerManager = tramTrackerManager
     self.tramTrackerController = tramTrackerController
 }
 
 func fetchUpcomingPredictedArrivals(forStopId stopId: String) async throws -> [PredictedArrival] {
     // Minimal stub implementation for testing.
     return []
 }
}
EOF

# Create a realistic PredictedArrival.swift file in MockFiles/Model/
cat << 'EOF' > MockFiles/Model/PredictedArrival.swift
//
//  PredictedArrival.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

struct Tram {
 let vehicleNumber: Int
 let isAirConditioned: Bool
}

struct PredictedArrival {
 let tram: Tram
 let routeNumber: String
 let predictedArrivalDateTime: Date
}
EOF

# Ensure that the TramTrackerViewModel.swift file is the most recently modified.
sleep 1
touch MockFiles/ViewModel/TramTrackerViewModel.swift

# Run the generate-prompt.sh script (the script uses the Git root, which is our TMP_DIR).
run bash generate-prompt.sh
[ "$status" -eq 0 ]

# Extract the final list of file basenames from the output.
final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')

# Define the expected final list.
expected_list=$(echo -e "PredictedArrival.swift\nTramTrackerUseCase.swift\nTramTrackerViewModel.swift" | sort)
final_list_sorted=$(echo "$final_list" | sort)

[ "$final_list_sorted" = "$expected_list" ]

# Assert that the success section includes the expected TODO instruction.
[[ "$output" == *"// TODO: - fetch these in parallel and populate the respective published varss"* ]]
}
 
@test "generate-prompt.sh outputs only the expected files when many extra files exist with realistic content" {
# Remove any default Swift files created by setup.
rm -f Test.swift Another.swift

# --- Create the minimal required files ---

# Create directory structure for the expected files.
mkdir -p MockFiles/Model
mkdir -p MockFiles/TramTracker
mkdir -p MockFiles/ViewModel

# Create TramTrackerViewModel.swift (expected file with unique TODO).
cat << 'EOF' > MockFiles/ViewModel/TramTrackerViewModel.swift
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
                // (Rest of implementation omitted for brevity)
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
EOF

# Create TramTrackerUseCase.swift (expected file with its TODO comment).
cat << 'EOF' > MockFiles/TramTracker/TramTrackerUseCase.swift
//
//  TramTrackerUseCase.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

enum FormError: Error {
    case badCapture
}

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
        let capturedUsername = "foo"
        let capturedPassword = "bar"
        
        if capturedUsername == capturedPassword {
            throw FormError.badCapture
        }
                
        if capturedUsername == capturedPassword {
            print("Although the captured username is actually the captured password, we are knowingly going to show it as plain text") // This is unethical and possibly illegal
        }
        
        return try await tramTrackerController.fetchPredictedArrivals(forStopId: stopId, token: token)
    }
    
}
EOF

# Create PredictedArrival.swift (expected file).
cat << 'EOF' > MockFiles/Model/PredictedArrival.swift
//
//  PredictedArrival.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

struct PredictedArrival {
    let tram: Tram
    let routeNumber: String
    let predictedArrivalDateTime: Date
}
EOF

# Ensure that the TramTrackerViewModel.swift file is the most recently modified.
sleep 1
touch MockFiles/ViewModel/TramTrackerViewModel.swift

# --- Create extra Swift files that should NOT be included ---

# Create an App file.
mkdir -p MockFiles/App
cat << 'EOF' > MockFiles/App/TramTrackerSwiftUIApp.swift
//
//  TramTrackerSwiftUIApp.swift
//  TramTrackerSwiftUI
//
//  Created on 16/3/2024.
//

import SwiftUI

@main
struct TramTrackerSwiftUIApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
EOF

# Create DeviceTokenResponse.swift.
mkdir -p MockFiles/Network
cat << 'EOF' > MockFiles/Network/DeviceTokenResponse.swift
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
EOF

# Create NextPredictedRoutesCollectionResponse.swift.
cat << 'EOF' > MockFiles/Network/NextPredictedRoutesCollectionResponse.swift
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
EOF

# Create TramTrackerService.swift.
cat << 'EOF' > MockFiles/TramTracker/TramTrackerService.swift
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
EOF

# Create HTTPClient.swift.
cat << 'EOF' > MockFiles/Network/HTTPClient.swift
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
EOF

# Create TramTrackerController.swift.
cat << 'EOF' > MockFiles/TramTracker/TramTrackerController.swift
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
EOF

# Create TramTrackerManager.swift.
cat << 'EOF' > MockFiles/TramTracker/TramTrackerManager.swift
//
//  TramTrackerManager.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

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
EOF

# Create Array Extensions.swift.
mkdir -p MockFiles/Extensions
cat << 'EOF' > "MockFiles/Extensions/Array Extensions.swift"
//
//  Array Extensions.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

extension Array {
    func safeElement(at index: Index) -> Element? {
        return indices.contains(index) ? self[index] : nil
    }
}
EOF

# Create Tram.swift.
cat << 'EOF' > MockFiles/Model/Tram.swift
//
//  Tram.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

struct Tram {
    let vehicleNumber: Int // TODO: maybe make String b/c not going to do mathematics on it
    let isAirConditioned: Bool
}
EOF

# Create ContentView.swift.
cat << 'EOF' > MockFiles/ContentView.swift
//
//  ContentView.swift
//  TramTrackerSwiftUI
//
//  Created on 16/3/2024.
//

import SwiftUI

struct ContentView: View {
    @StateObject private var viewModel = TramTrackerViewModel()
    
    var body: some View {
        NavigationView {
            VStack {
                if let errorMessage = viewModel.errorMessage {
                    ErrorView(errorMessage: errorMessage)
                } else if viewModel.isLoading {
                    LoadingView()
                } else if let northBoundPredictedArrivals = viewModel.northBoundPredictedArrivals,
                          let southBoundPredictedArrivals = viewModel.southBoundPredictedArrivals {
                    TramArrivalsListView(
                        northBoundPredictedArrivals: northBoundPredictedArrivals,
                        southBoundPredictedArrivals: southBoundPredictedArrivals,
                        northStopIdentifier: viewModel.northStopIdentifier,
                        southStopIdentifier: viewModel.southStopIdentifier
                    )
                } else {
                    InformationView()
                }
            }
            .padding()
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Clear") {
                        viewModel.clearPredictedArrivals()
                    }
                    .disabled(!viewModel.hasLoaded || viewModel.isLoading)
                    .tint(Color.red)
                    .accessibilityLabel("Clear Arrivals")
                    .accessibilityHint("Clears the list of predicted upcoming tram arrivals.")
                }
            }
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Load") {
                        viewModel.loadPredictedArrivals()
                    }
                    .disabled(viewModel.isLoading)
                    .tint(Color.accentColor)
                    .accessibilityLabel("Load Upcoming Tram Arrivals")
                    .accessibilityHint("Loads and shows the predicted upcoming tram arrivals.")
                }
            }
        }
    }
}

// MARK: - Lifecycle Views

struct InformationView: View {
    var body: some View {
        Text("💡 Press \"Load\" to show upcoming arrivals")
            .foregroundColor(.secondary)
            .frame(maxWidth: .infinity, alignment: .center)
            .padding()
            .multilineTextAlignment(.center)
            .lineLimit(nil)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityLabel("Information")
            .accessibilityValue("Press Load to show upcoming arrivals")
            .accessibilityHint("Pressing the 'Load' button, located in the top right-hand corner of the screen, will load and show the times when trams are expected to arrive. Use the navigation bar at the top to find the 'Load' button.")
    }
}

struct LoadingView: View {
    var body: some View {
        ProgressView()
            .progressViewStyle(CircularProgressViewStyle())
            .frame(maxWidth: .infinity, alignment: .center)
            .padding()
            .accessibilityLabel("Loading")
            .accessibilityHint("Indicates that tram arrival times are currently loading.")
    }
}

struct ErrorView: View {
    let errorMessage: String

    var body: some View {
        Text(errorMessage)
            .foregroundColor(.red)
            .frame(maxWidth: .infinity, alignment: .center)
            .padding()
            .multilineTextAlignment(.center)
            .lineLimit(nil)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityLabel("Error Message")
            .accessibilityValue(errorMessage)
            .accessibilityHint("Displays an error message related to tram arrival times.")
    }
}

// MARK: - Main View Components

struct TramArrivalsListView: View {
    let northBoundPredictedArrivals: [PredictedArrival]
    let southBoundPredictedArrivals: [PredictedArrival]
    
    let northStopIdentifier: String
    let southStopIdentifier: String

    var body: some View {
        List {
            Section(header: TramArrivalSectionHeaderView(title: "Northbound Trams (Stop \(northStopIdentifier))")) {
                ForEach(northBoundPredictedArrivals) { arrival in
                    TramArrivalView(arrival: arrival)
                }
            }
            Section(header: TramArrivalSectionHeaderView(title: "Southbound Trams (Stop \(southStopIdentifier))")) {
                ForEach(southBoundPredictedArrivals) { arrival in
                    TramArrivalView(arrival: arrival)
                }
            }
        }
        .listStyle(PlainListStyle())
        .background(Color.clear)
    }
}

struct TramArrivalSectionHeaderView: View {
    let title: String

    var body: some View {
        Text(title)
            .font(.headline)
            .padding(.top)
            .accessibilityHint("Header for a section showing upcoming tram arrivals.")
    }
}

struct TramArrivalView: View {
    let arrival: PredictedArrival
    
    var formattedArrivalTime: String {
        let arrivalFormatter = DateFormatter()
        arrivalFormatter.dateFormat = "h:mm a"
        arrivalFormatter.amSymbol = "am"
        arrivalFormatter.pmSymbol = "pm"
        return arrivalFormatter.string(from: arrival.predictedArrivalDateTime).lowercased()
    }
    
    var timeDifferenceString: String {
        let now = Date()
        let calendar = Calendar.current
        let diff = calendar.dateComponents([.minute], from: now, to: arrival.predictedArrivalDateTime)
        
        if let minute = diff.minute, minute < 60 {
            if minute == 1 {
                return "in one minute"
            } else {
                return "in \(minute) minutes"
            }
        } else if let minute = diff.minute {
            let hour = minute / 60
            let remainingMinutes = minute % 60
            if hour == 1 && remainingMinutes == 0 {
                return "in one hour"
            } else if hour > 1 && remainingMinutes == 0 {
                return "in \(hour) hours"
            } else if hour == 1 {
                return "in 1 hour and \(remainingMinutes) minutes"
            } else {
                return "in \(hour) hours and \(remainingMinutes) minutes"
            }
        } else {
            return "Unknown arrival"
        }
    }
    
    var accessibilityText: String {
        "Route \(arrival.routeNumber), arriving \(timeDifferenceString) at \(formattedArrivalTime)."
    }
    
    var body: some View {
        VStack(alignment: .leading) {
            Text("Route \(arrival.routeNumber)")
                .font(.subheadline)
            
            Text("Arriving \(timeDifferenceString) @ \(formattedArrivalTime)")
                .font(.footnote)
                .foregroundColor(.secondary)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityText)
        .accessibilityHint("Shows the arrival time and route number for a tram.")
    }
}

// MARK: - Convenience

extension PredictedArrival: Identifiable {
    var id: String { "\(routeNumber) \(tram.vehicleNumber) \(predictedArrivalDateTime.timeIntervalSinceReferenceDate)" }
}

#Preview {
    ContentView()
}
EOF

  # --- Run the generate-prompt.sh script and assert results ---

  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  # Extract the final list of file basenames (printed between "Files (final list):" and the next separator).
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
  
  # The expected final list should be exactly these three files.
  expected_list=$(echo -e "PredictedArrival.swift\nTramTrackerUseCase.swift\nTramTrackerViewModel.swift" | sort)
  final_list_sorted=$(echo "$final_list" | sort)
  
  expected_count=3
  actual_count=$(echo "$final_list_sorted" | wc -l | tr -d ' ')
  [ "$actual_count" -eq "$expected_count" ]
  [[ "$final_list_sorted" == *"PredictedArrival.swift"* ]]
  [[ "$final_list_sorted" == *"TramTrackerUseCase.swift"* ]]
  [[ "$final_list_sorted" == *"TramTrackerViewModel.swift"* ]]

  # Assert that the success section includes the expected unique TODO instruction.
  [[ "$output" == *"// TODO: - fetch these in parallel and populate the respective published varss"* ]]
}

@test "generate-prompt.sh exits with error if multiple markers found in prompt generated from realistic file content" {
  # Create a temporary directory for the test
  tmp_dir="$(mktemp -d)"
  cd "$tmp_dir"

  # Initialize a Git repository so that get_git_root succeeds
  git init -q .

  # Create a directory structure mimicking your repository layout.
  mkdir -p MockFiles/Model MockFiles/TramTracker MockFiles/ViewModel

  # Create a realistic TramTrackerViewModel.swift file in MockFiles/ViewModel/
  # This file contains the unique TODO instruction that the script should select.
  cat << 'EOF' > MockFiles/ViewModel/TramTrackerViewModel.swift
//
//  TramTrackerViewModel.swift
//  TramTrackerSwiftUI
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
 
 // MARK: - Properties
 
 private let useCase: TramTrackerUseCasing
 
 // MARK: - Constants
 
 private enum StopIdentifier {
     static let north = "4055"
     static let south = "4155"
 }
 
 // MARK: - Life-cycle
 
 init(useCase: TramTrackerUseCasing = TramTrackerUseCase()) {
     self.useCase = useCase
 }
 
 // MARK: - Public functions
 
 func loadPredictedArrivals() {
     self.isLoading = true
     self.errorMessage = nil
     
     Task {
         do {
             // TODO: - example only 1
             async let fetchedNorthBoundPredictedArrivals = try useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.north)
             async let fetchedSouthBoundPredictedArrivals = try useCase.fetchUpcomingPredictedArrivals(forStopId: StopIdentifier.south)
             // (Rest of implementation omitted for brevity)
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
EOF

  # Create a realistic TramTrackerUseCase.swift file in MockFiles/TramTracker/
  # This file contains an alternate TODO that should be considered offending.
  cat << 'EOF' > MockFiles/TramTracker/TramTrackerUseCase.swift
//
//  TramTrackerUseCase.swift
//  TramTrackerSwiftUI
//

import Foundation

// TODO: - example only 2

protocol TramTrackerUseCasing {
    func fetchUpcomingPredictedArrivals(forStopId stopId: String) async throws -> [String]
}
EOF

  # Create a simple PredictedArrival.swift file in MockFiles/Model/
  cat << 'EOF' > MockFiles/Model/PredictedArrival.swift
//
//  PredictedArrival.swift
//  TramTrackerSwiftUI
//

import Foundation

struct PredictedArrival { }
EOF

  # Make sure the ViewModel file is the most recently modified.
  sleep 1
  touch MockFiles/ViewModel/TramTrackerViewModel.swift

  # Run the harness script (which calls the binary generate_prompt)
  run bash generate-prompt.sh

  # Check that the script exited with a nonzero status.
  [ "$status" -ne 0 ]

  # Check that the error message about multiple markers is printed.
  echo "$output" | grep -q "Multiple // TODO: - markers found. Exiting."

  # Optionally, check that all offending markers (except the very last CTA marker) are printed.
  offending_markers=$(echo "$output" | grep "TODO: -" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  last_marker=$(echo "$offending_markers" | tail -n 1)
  [ "$last_marker" != "// TODO: -" ]

  # Clean up the temporary directory.
  rm -rf "$tmp_dir"
}
 
# --- New tests for --include-references functionality using Rust binaries ---
 
@test "extract_enclosing_type helper extracts the correct type from a Swift file" {
    # Create a temporary file with a type definition and a TODO instruction.
    echo "class MySpecialClass {}" > tempTodo.swift
    echo "// TODO: - Implement feature" >> tempTodo.swift
    run "$TMP_DIR/rust/target/release/extract_enclosing_type" "tempTodo.swift"
    [ "$status" -eq 0 ]
    [ "$output" = "MySpecialClass" ]
    rm tempTodo.swift
}
 
@test "find-referencing-files helper finds referencing files for a given type" {
  # Create two files: one that references the type and one that does not.
  echo "let instance = MySpecialClass()" > tempRef.swift
  echo "print(\"No reference here\")" > tempNonRef.swift

  # Run the new Rust binary for finding referencing files.
  run "./rust/target/release/find_referencing_files" "MySpecialClass" "."
  [ "$status" -eq 0 ]

  # Capture the output directly (which is now a list of file paths).
  refList="$output"

  # Verify that the list includes tempRef.swift but not tempNonRef.swift.
  [[ "$refList" == *"tempRef.swift"* ]]
  [[ "$refList" != *"tempNonRef.swift"* ]]

  # Clean up only the test-created files.
  rm tempRef.swift tempNonRef.swift
}
 
@test "generate-prompt.sh with --include-references includes referencing files" {
    # Remove any default Test.swift so that our new TODO file is the only valid one.
    rm -f Test.swift
    # Create a new TODO file that defines a type.
    cat << 'EOF' > Todo.swift
import Foundation
// TODO: - Implement special feature
class MyTodoClass {}
EOF
    # Create a referencing file that mentions MyTodoClass.
    cat << 'EOF' > Reference.swift
import Foundation
let ref = MyTodoClass()
EOF
    run bash generate-prompt.sh --include-references
    [ "$status" -eq 0 ]
    final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
    [[ "$final_list" == *"Todo.swift"* ]]
    [[ "$final_list" == *"Reference.swift"* ]]
    # Clean up the temporary files.
    rm Todo.swift Reference.swift
}

@test "generate-prompt.sh extracts enclosing function context and excludes content before markers" {
  # Create a temporary JS file with sample content.
  cat << 'EOF' > FunctionTest.js
const someExampleConstant = 42;

// v

const anotherExampleConstant = 99;

// ^

Parse.Cloud.define("getDashboardData", async (request) => {
    
    // TODO: - helllo
    
    var environment = require("./environment.js");
    var _ = getUnderscore();
    
    var currentUserObjectId = request.params.currentUserObjectId;
    var currentUserGlobal;
    var hiddenPeopleGlobal;
    var timeAgoGlobal = new Date(new Date().getTime() - (24 * 60 * 60 * 1000));
    var resultDictionaryGlobal;
    
});
EOF

  # Set the environment variable so the script knows which file contains the TODO.
  export TODO_FILE_BASENAME=$(basename "FunctionTest.js")

  # Create a temporary found-files list including our file.
  echo "FunctionTest.js" > found_files.txt

  # Run the generate-prompt.sh script.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  # Read the content copied to the dummy clipboard.
  clipboard_content=$(cat clipboard.txt)

  # Assert that the function block is included (starting with Parse.Cloud.define).
  [[ "$clipboard_content" == *'Parse.Cloud.define("getDashboardData", async (request) => {'* ]]

  # Assert that the TODO comment inside the function block is present.
  [[ "$clipboard_content" == *"// TODO: - helllo"* ]]

  # Assert that extra context (enclosing function context) was appended.
  [[ "$clipboard_content" == *"// Enclosing function context:"* ]]

  # Assert that content from before the substring markers is NOT included.
  [[ "$clipboard_content" != *"const someExampleConstant = 42;"* ]]
}
