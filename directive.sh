Can we add support for "substring markers" ...by which I mean, if there's something like this in a file that will be included in our prompt:

final class MediaItemViewController: UIViewController, UIScrollViewDelegate {
    
    var mediaItem: MediaItem? {
        didSet {
            let data: Data?
            if mediaItem?.mediaItemType == .video {
                data = mediaItem?.thumbnailData
                playButtonView.isHidden = false
            } else {
                data = mediaItem?.data
                playButtonView.isHidden = true
            }
            if let data = data, let image = UIImage(data: data) {
                imageView.image = image
                errorLabel.isHidden = true
                if let username = mediaItem?.username {
                    attributionLabel.text = "- \(username)"
                } else {
                    attributionLabel.text = nil
                }
                // Set the caption text (wrapped in quotation marks) if available.
                if let caption = mediaItem?.caption, !caption.isEmpty {
                    captionLabel.text = "“\(caption)”"
                } else {
                    captionLabel.text = nil
                }
                DispatchQueue.main.async {
                    self.view.setNeedsLayout()
                    self.view.layoutIfNeeded()
                }
            } else {
                imageView.image = nil
                errorLabel.isHidden = false
                attributionLabel.text = nil
                captionLabel.text = nil
            }
        }
    }
    
    // v
    var zoomAndPanStateChanged: ((Bool, Bool) -> Void)?
    var onSingleTapImage: (() -> Void)?
    // ^
    var onTapProfile: (() -> Void)?
    var onReply: (() -> Void)?
    var onFinishedWatchingVideo: ((Double?) -> Void)?
    var onClose: (() -> Void)?
    
    // v
    private var isZoomedIn: Bool = false {
        didSet {
            zoomAndPanStateChanged?(isZoomedIn, endedPanningAtEdge)
            swipeDownGestureRecognizer.isEnabled = !isZoomedIn
        }
    }
    // ^
    
    private var endedPanningAtEdge: Bool = false {
        didSet {
            zoomAndPanStateChanged?(isZoomedIn, endedPanningAtEdge)
        }
    }
    
    var exposedScrollView: UIScrollView { scrollView }
    
    // v
    private var player: AVPlayer?
    // ^
    private var playingVideo: Bool?
    private var lastPlaybackTime: Double?
    
    // and more code here that I'm not including
    
}

That we only include, from that file, the text between the opening // v marker and the // ^ closing marker...as in, it's just what we do as per normal, except we only include the text between the opening and closing substring markers...but we also should put:

// ...

where the "filtered out" text would have gone

So, running the script for the above input should yield:

The contents of MediaItemViewController.swift is as follows:

// ...

    var zoomAndPanStateChanged: ((Bool, Bool) -> Void)?
    var onSingleTapImage: (() -> Void)?
    
// ...

    private var isZoomedIn: Bool = false {
        didSet {
            zoomAndPanStateChanged?(isZoomedIn, endedPanningAtEdge)
            swipeDownGestureRecognizer.isEnabled = !isZoomedIn
        }
    }
    
// ...

    private var player: AVPlayer?
    
// ...


Notice how all the not-included code (i.e. code outside of the substring markers) has been replaced by a placeholder indicator i.e. // ... with a line break above and below


