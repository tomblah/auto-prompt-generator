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

// This is to allow ForEach iterating in the List

// NB: A little bit annoying that in Swift you can't have private extensions that add conformance to a protocol...because this really is only for ContentView's benefit and therefore should be declared private

extension PredictedArrival: Identifiable {
    
    var id: String { "\(routeNumber) \(tram.vehicleNumber) \(predictedArrivalDateTime.timeIntervalSinceReferenceDate)" }
    
}

#Preview {
    ContentView()
}

