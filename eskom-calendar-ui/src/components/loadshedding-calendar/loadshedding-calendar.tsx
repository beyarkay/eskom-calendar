import FullCalendar from "@fullcalendar/react"; // must go before plugins
import dayGridPlugin from "@fullcalendar/daygrid"; // a plugin!
import iCalendarPlugin from "@fullcalendar/icalendar";
export interface ILoadsheddingCalendar {
  eventData?: any[];
  eventCalendarName?: string;
}

function LoadsheddingCalendar({
  eventData,
  eventCalendarName,
}: ILoadsheddingCalendar) {
  var eventDataItem: any = eventData;
  if (eventCalendarName) {
    eventDataItem = {
      //Proxy URL to overcome CORS.
      url: `https://quotemanagerapi20220701201338.azurewebsites.net/api/Calendar/Get/${eventCalendarName}`,
      format: "ics",
    };
  }

  return (
    <FullCalendar
      themeSystem="bootstrap5"
      plugins={[dayGridPlugin, iCalendarPlugin]}
      initialView="dayGridMonth"
      weekends={false}
      events={eventDataItem}
    />
  );
}

export default LoadsheddingCalendar;
