use libpulse_binding as pulse;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet};
use libpulse_binding::proplist::Proplist;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Volume {
    pub value: u32,
    pub muted: bool,
}

pub struct PulseManager {
    mainloop: pulse::mainloop::threaded::Mainloop,
    context: pulse::context::Context,
}

impl PulseManager {
    pub fn new() -> anyhow::Result<Self> {
        let mut proplist =
            Proplist::new().ok_or_else(|| anyhow::anyhow!("Proplist creation failed"))?;
        proplist
            .set_str(pulse::proplist::properties::APPLICATION_NAME, "PAnotify")
            .unwrap();

        let mut mainloop = pulse::mainloop::threaded::Mainloop::new()
            .ok_or_else(|| anyhow::anyhow!("Mainloop creation failed"))?;

        let mut context =
            pulse::context::Context::new_with_proplist(&mainloop, "PAnotify", &proplist)
                .ok_or_else(|| anyhow::anyhow!("Context creation failed"))?;

        context.connect(None, pulse::context::FlagSet::NOFLAGS, None)?;
        mainloop.start()?;

        while context.get_state() != pulse::context::State::Ready {
            mainloop.wait();
        }

        Ok(Self { mainloop, context })
    }

    pub fn get_default_sink_volume(&mut self) -> anyhow::Result<Volume> {
        let sink_name = self.get_default_sink_name()?;
        self.get_sink_volume(&sink_name)
    }

    pub fn get_default_sink_name(&mut self) -> anyhow::Result<Box<str>> {
        let result = Rc::new(RefCell::new(None));

        let op = self.context.introspect().get_server_info({
            let result = Rc::clone(&result);
            move |info| {
                *result.borrow_mut() = info.default_sink_name.as_ref().map(|n| n.as_ref().into());
            }
        });

        self.wait_for_operation(op)?;
        result
            .borrow_mut()
            .take()
            .ok_or_else(|| anyhow::anyhow!("No default sink"))
    }

    pub fn get_sink_volume(&mut self, sink_name: &str) -> anyhow::Result<Volume> {
        let result = Rc::new(RefCell::new(None));

        let op = self.context.introspect().get_sink_info_by_name(sink_name, {
            let result = Rc::clone(&result);
            move |sink_list| {
                if let ListResult::Item(item) = sink_list {
                    *result.borrow_mut() = Some(Volume {
                        value: item
                            .volume
                            .max()
                            .print()
                            .trim_end_matches('%')
                            .trim()
                            .parse()
                            .unwrap_or_default(),
                        muted: item.mute,
                    });
                }
            }
        });

        self.wait_for_operation(op)?;
        result
            .borrow_mut()
            .take()
            .ok_or_else(|| anyhow::anyhow!("Sink not found"))
    }

    pub fn wait_for_operation<T>(
        &mut self,
        op: pulse::operation::Operation<T>,
    ) -> anyhow::Result<()>
    where
        T: ?Sized,
    {
        while op.get_state() != pulse::operation::State::Done {
            self.mainloop.wait();
        }
        Ok(())
    }

    pub fn subscribe(&mut self, mask: InterestMaskSet) {
        self.context.subscribe(mask, |_| {});
    }

    pub fn set_subscription_callback<F>(&mut self, callback: F)
    where
        F: Fn(Option<Facility>, Option<pulse::context::subscribe::Operation>, u32) + 'static,
    {
        self.context
            .set_subscribe_callback(Some(Box::new(callback)));
    }
}
