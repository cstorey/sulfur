# Implemented parts of the spec

* 6 Protocol
  * [ ] 6.6 Errors
    - Error codes need to be interpreted.
* 7 Capabilities
  * [ ] Expose known capabilities via session?
  * [ ] 7.1 Proxy
  * [ ] 7.2 Processing capabilities
* 8 Sessions
  * [x] 8.1 New Session
  * [x] 8.2 Delete Session
  * [x] 8.3 Status
  * [x] 8.4 Get Timeouts
  * [x] 8.5 Set Timeouts
* 9 Navigation
  * [x] 9.1 Navigate To
  * [x] 9.2 Get Current URL
  * [x] 9.3 Back
  * [x] 9.4 Forward
  * [x] 9.5 Refresh
  * [x] 9.6 Get Title
* [ ] 10 Contexts
  * [x] 10.1 Get Window Handle
  * [x] 10.2 Close Window
  * [x] 10.3 Switch To Window
  * [x] 10.4 Get Window Handles
  * [ ] 10.5 Create Window
    * Not supported in most implementations?
  * [x] 10.6 Switch To Frame
  * [x] 10.7 Switch To Parent Frame
  * [ ] 10.8 Resizing and positioning windows
    * [ ] 10.8.1 Get Window Rect
    * [ ] 10.8.2 Set Window Rect
    * [ ] 10.8.3 Maximize Window
    * [ ] 10.8.4 Minimize Window
    * [ ] 10.8.5 Fullscreen Window
* 11 Elements
  * 11.2 Retrieval
    * 11.2.1 Locator strategies
      * [x] 11.2.1.1 CSS selectors
      * [ ] 11.2.1.2 Link text
      * [ ] 11.2.1.3 Partial link text
      * [ ] 11.2.1.4 Tag name
      * [ ] 11.2.1.5 XPath
    * [x] 11.2.2 Find Element
    * [x] 11.2.3 Find Elements
    * [x] 11.2.4 Find Element From Element
    * [x] 11.2.5 Find Elements From Element
    * [ ] 11.2.6 Get Active Element
  * 11.3 State
    * [ ] 11.3.1 Is Element Selected
    * [ ] 11.3.2 Get Element Attribute
    * [ ] 11.3.3 Get Element Property
    * [ ] 11.3.4 Get Element CSS Value
    * [x] 11.3.5 Get Element Text
    * [x] 11.3.6 Get Element Tag Name
    * [ ] 11.3.7 Get Element Rect
    * [ ] 11.3.8 Is Element Enabled
  * 11.4 Interaction
    * [x] 11.4.1 Element Click
    * [x] 11.4.2 Element Clear
    * [x] 11.4.3 Element Send Keys
* [ ] 12 Document
  * [ ] 12.1 Get Page Source
  * [ ] 12.2 Executing Script
    * [ ] 12.2.1 Execute Script
    * [ ] 12.2.2 Execute Async Script
* [ ] 13 Cookies
  * [ ] 13.1 Get All Cookies
  * [ ] 13.2 Get Named Cookie
  * [ ] 13.3 Add Cookie
  * [ ] 13.4 Delete Cookie
  * [ ] 13.5 Delete All Cookies
* [ ] 14 Actions
  * Most implementations seem to wrap these in a higher level interface.
  * [ ] 14.1 Input sources
    * [ ] 14.1.1 Sources
    * [ ] 14.1.2 State
  * [ ] 14.2 Ticks
  * [ ] 14.3 Processing actions
  * [ ] 14.4 Dispatching actions
    * [ ] 14.4.1 General actions
    * [ ] 14.4.2 Keyboard actions
    * [ ] 14.4.3 Pointer actions
  * [ ] 14.5 Perform Actions
  * [ ] 14.6 Release Actions
* [ ] 15 User prompts
  * [ ] 15.1 Dismiss Alert
  * [ ] 15.2 Accept Alert
  * [ ] 15.3 Get Alert Text
  * [ ] 15.4 Send Alert Text
* [ ] 16 Screen capture
  * [ ] 16.1 Take Screenshot
  * [ ] 16.2 Take Element Screenshot
