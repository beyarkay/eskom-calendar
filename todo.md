To Do
-----

# Custom server

Look into server/serverless experience on various cloud providers

- Can oracle cloud be setup reliably?
- Can it monitor the downloads of machine_friendly every hour (or every minute)?
- How will the existing calendars be deprecated?
- Some way of *pushing* updates to people
- An actual API (which I could theoretically monetize)
- Bots. Bots for telegram, whatsapp, etc

## What the server should do

Important notes:

- Must accept an API key (so I can do admin things to the DB)
- Must accept a JSON string, not query parameters/HTML forms
- Must use OpenAPI
- Must be versioned
- Should also emit calendars like https://eskomcalendar.co.za/v1/western-cape-stellenbosch.ics


bio / about us - for joel

Getting loadshedding info (send data just as a JSON blob)
- curl https://eskomcalendar.co.za/v1
- curl --form calendar=my-calendar.ics --form start=2022-01-01 finsh=2022-01-02 https://eskomcalendar.co.za/v1/schedules


# Contributor experience
- Update the instructions in manually_specified.yaml

# Documentation
- Add a list of people who use eskom-calendar
- Add stats like time-to-reply
- Add FAQ onto the website
- Add table of comparisons to GitHub, comparing eskcal to ESP and others
- Add Creative Commons license to machine_friendly.csv and others
- Add "Recommended Attribution" page in README, specifying the appropriate ways
  of using machine_friendly.csv
- Pretty up the PR editor

# Performance improvements

- Maybe the YAML file area_metadata.yaml can be shrunk by
    - removing unnecessary double quotes
    - removing unnecessary indentation
    - Removing unnecessary spaces

# Missing area schedules

- Gauteng Tshwane
    - Queenswood Ext 1 (8

- Free State, Bloemfontein
    - Municipality: Mangaung Group 4
        - Area: Universitas

- Western Cape
    - Bothasig
    - Burgundy Estate (apparently the same as bothasig)

- Easter cape, East London
    - Buffalo city
        - Nahoon valley

- Apparently East London itself isn't added? https://www.reddit.com/r/southafrica/comments/w3hq7v/get_loadshedding_schedules_in_your_calendar/ip7v4y9/
- Agter-paarl?
  [this](http://ww1.drakenstein.gov.za/cc/Pages/Key-to-Eskom-load-shedding.aspx)
  says  its on block 11 but ESP says block 10

# Sources:

- [ ] [Nelson Mandela Bay](https://nelsonmandelabay.gov.za/page/loadshedding)
    - NMB does not publish a repeating cycle, but rather it has a 20-day cycle
      which it announces ~3 months at a time, and there's no guarantee that it
      will keep to that schedule
- [x] [eThekwini](https://www.durban.gov.za/pages/residents/load-shedding) AKA Durban
    - Uses a 7-day cycle based on the days of the week.
- [x] [Manguang](https://www.centlec.co.za/LoadShedding/LoadSheddingDocuments): AKA Bloemfontein
    - also at http://www.mangaung.co.za/
    - Uses a 7-day cycle based on the days of the week.
- [x] [City of Ekurhuleni](https://www.ekurhuleni.gov.za/residents/load-shedding-schedules.html)
- [x] [Buffalo City](https://www.buffalocity.gov.za/folder.php?id=CA4FCA0)
    - Shouldn't be too tricky
    - uses a 31-day cycle
- [x] [City of Johannesburg](https://www.citypower.co.za/customers/Pages/Load_Shedding_Downloads.aspx)
- [x] [City of Tshwane](https://www.tshwane.gov.za/?page_id=1124#293-293-wpfd-top)
- [x] [City of Cape Town](PDF)(http://resource.capetown.gov.za/documentcentre/Documents/Procedures%2C%20guidelines%20and%20regulations/Load_Shedding_All_Areas_Schedule_and_Map.pdf)

- http://www.tshwane.gov.za/sites/Departments/Public-works-and-infrastructure/Pages/Load-Shedding.aspx
- https://www.citypower.co.za/customers/Pages/Load_Shedding_Downloads.aspx
- https://www.ekurhuleni.gov.za/residents/load-shedding-schedules.html
- http://resource.capetown.gov.za/documentcentre/Documents/Procedures%2C%20guidelines%20and%20regulations/Load_Shedding_All_Areas_Schedule_and_Map.pdf
- https://nelsonmandelabay.gov.za/page/loadshedding
- http://www.durban.gov.za/City_Services/electricity/Load_Shedding/Pages/default.aspx
- http://www.centlec.co.za/Pages/Page/Loadshedding
- https://www.buffalocity.gov.za/notice.php?id=4JVjjqq9G

